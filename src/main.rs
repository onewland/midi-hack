use std::error::Error;
use std::io::stdin;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::{cmp::max, thread::JoinHandle, time::Duration};

use clap::Parser;
use log::{debug, info, trace};

use midi_hack::key_handler::{ControlMessage, KeyDb};
use midi_hack::midi::{KeyMessage, KNOWN_MESSAGE_TYPES};
use midi_hack::practice_program::{
    CircleOfFourthsPracticeProgram, EarTrainingPracticeProgram, FreePlayPracticeProgram,
    PracticeProgram,
};
use midir::{Ignore, MidiInput, MidiOutput};

const HEARTBEATS_PER_AUTO_NEW_RUN: usize = 100;

// #[derive(Clone)]
struct KeyLogAndDispatch {
    key_db: Arc<KeyDb>,
    most_recent_insert: u64,
    keypress_listeners: Vec<Box<dyn RunEndListener + Send>>,
    heartbeat_count: usize,
    program_sender: SyncSender<KeyMessage>,
}

impl KeyLogAndDispatch {
    fn new(program_sender: SyncSender<KeyMessage>, key_db: Arc<KeyDb>) -> KeyLogAndDispatch {
        return KeyLogAndDispatch {
            key_db,
            most_recent_insert: 0,
            keypress_listeners: Vec::new(),
            heartbeat_count: 0,
            program_sender,
        };
    }

    fn accept(&mut self, message: KeyMessage) {
        self.key_db.push_msg(message);
        self.most_recent_insert = max(message.timestamp, self.most_recent_insert);
        self.call_listeners(message);
        self.program_sender.send(message).unwrap();
    }

    fn end_run(&mut self) {
        info!("{}", "[new run]");
        self.key_db.clear();
        self.heartbeat_count = 0;
        self.print()
    }

    fn call_listeners(&mut self, message: KeyMessage) {
        let mut hit_end = false;
        for listener in &self.keypress_listeners {
            hit_end = hit_end
                || listener
                    .as_ref()
                    .on_keypress(Arc::clone(&self.key_db), message);
        }
        if hit_end {
            self.end_run()
        }
    }

    fn handle_control_message(&mut self, msg: ControlMessage) {
        match msg {
            ControlMessage::Heartbeat => self.heartbeat(),
            ControlMessage::NewRun => self.end_run(),
            ControlMessage::Print => self.print(),
        }
    }

    fn heartbeat(&mut self) {
        if self.heartbeat_count > HEARTBEATS_PER_AUTO_NEW_RUN {
            self.end_run()
        }
        self.heartbeat_count += 1;
    }

    fn print(&self) {
        print!(
            "KeyBuffer [ most_recent_insert = {} ] [ keys = ",
            self.most_recent_insert
        );
        let mut last_msg: Option<KeyMessage> = None;
        self.key_db.flat_message_log().iter().for_each(|msg| {
            // print rest time since prior note
            match last_msg {
                None => (),
                Some(prev) => print!("{} ", msg.timestamp - prev.timestamp),
            }
            // print note
            msg.print();

            last_msg = Some(*msg);
        });
        println!("]");
        self.key_db.print_holds();
    }

    pub(crate) fn start_recv_loop(
        mut self,
        playback_receiver: Receiver<KeyMessage>,
        control_receiver: Receiver<ControlMessage>,
    ) -> JoinHandle<()> {
        return std::thread::spawn(move || {
            loop {
                match playback_receiver.recv_timeout(std::time::Duration::from_nanos(100)) {
                    Ok(message) => self.accept(message),
                    Err(_recv_timeout_error) => (), // this is fine
                };
                match control_receiver.recv_timeout(std::time::Duration::from_nanos(100)) {
                    Ok(message) => self.handle_control_message(message),
                    Err(_recv_timeout_error) => (), // this is fine
                }
            }
        });
    }
}

trait RunEndListener {
    // RunEndListener listens on runs for the end, if it returns
    // true it has detected an end of a run, false means that it has not
    fn on_keypress(&self, kmsg_log: Arc<KeyDb>, latest: KeyMessage) -> bool;
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    // Midi read setup
    let mut input = String::new();
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);
    let midi_out: MidiOutput = MidiOutput::new("midir_controller_client")?;
    let out_ports = midi_out.ports();
    debug!("{} ports in midi_out", midi_out.port_count());
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => panic!("no device found"),
        len => {
            assert!(len > cli.midi_device_port);
            let device_name = midi_in.port_name(&in_ports[cli.midi_device_port]).unwrap();
            println!(
                "Loading input port {}, friendly name: \"{}\"",
                cli.midi_device_port, device_name
            );
            sentry::configure_scope(|scope| scope.set_tag("midi_in_device", device_name));
            &in_ports[0]
        }
    };
    let out_port = match out_ports.len() {
        0 => None,
        len => {
            assert!(len > cli.midi_device_port);
            let device_name = midi_out
                .port_name(&out_ports[cli.midi_device_port])
                .unwrap();
            println!(
                "Loading output port {}, friendly name: \"{}\"",
                cli.midi_device_port, device_name
            );
            sentry::configure_scope(|scope| scope.set_tag("midi_out_device", device_name));
            Some(&out_ports[0])
        }
    };
    let midi_out_connection =
        out_port.map(|port| midi_out.connect(port, "midir-write-output").unwrap());

    info!("output connection established");

    // Listener setup
    let (playback_sender, playback_receiver) = sync_channel(1);
    let (control_sender, control_receiver) = sync_channel(10);
    let (program_sender, program_receiver) = sync_channel(10);
    // the size of this queue will impact number of simultaneous-sounding notes emitted
    // (e.g. if set to 1 you can never get a "chord sound")
    let (midi_out_sender, midi_out_receiver) = sync_channel::<KeyMessage>(5);

    let control_sender_tty = control_sender.clone();
    let control_sender_practice_program = control_sender.clone();
    let key_db = Arc::from(KeyDb::new(256));
    let key_reader_ro_copy = Arc::clone(&key_db);
    let key_reader = KeyLogAndDispatch::new(program_sender, key_db);
    match cli.practice_program.as_ref() {
        "circle-of-fourths" => {
            let program = CircleOfFourthsPracticeProgram::new(
                control_sender_practice_program,
                program_receiver,
                key_reader_ro_copy,
            );
            program.run();
        }
        "ear-training" => {
            assert!(
                midi_out_connection.is_some(),
                "functional MIDI out required for ear training"
            );
            let program = EarTrainingPracticeProgram::new(
                control_sender_practice_program,
                midi_out_sender,
                program_receiver,
                key_reader_ro_copy,
                true,
            );
            program.run();
        }
        &_ => {
            let program = FreePlayPracticeProgram::new(
                control_sender_practice_program,
                program_receiver,
                key_reader_ro_copy,
            );
            program.run();
        }
    };

    // Start the read loop
    let _conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |stamp, message, _| {
            if !KNOWN_MESSAGE_TYPES.contains(&message[0]) {
                println!(
                    "unknown message {}: {:?} (len = {})",
                    stamp,
                    message,
                    message.len()
                );
            }
            if message.len() == 3 {
                if message[0] == midi_hack::midi::KEY_UP || message[0] == midi_hack::midi::KEY_DOWN
                {
                    let parsed_message = KeyMessage::from_midi(stamp, message);
                    playback_sender.send(parsed_message).unwrap();
                }
            }
        },
        (),
    )?;

    std::thread::spawn(move || {
        const HEARTBEAT_LAPSE_SECONDS: u64 = 1;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(HEARTBEAT_LAPSE_SECONDS));
            control_sender.send(ControlMessage::Heartbeat).unwrap();
        }
    });

    key_reader.start_recv_loop(playback_receiver, control_receiver);
    midi_hack::time::start_timer();

    if midi_out_connection.is_some() {
        std::thread::spawn(move || {
            const WAIT_DELAY: Duration = std::time::Duration::from_secs(1);
            let mut midi_out = midi_out_connection.unwrap();
            info!("midi out receive loop started");
            loop {
                match midi_out_receiver.recv_timeout(WAIT_DELAY) {
                    Ok(message) => {
                        trace!("emitting {:?}", message);
                        midi_out.send(&message.encode())
                    }
                    Err(_recv_timeout_error) => Ok(()), // this is fine
                }
                .unwrap();
            }
        });
    }

    let mut stop_the_show = false;

    while !stop_the_show {
        input.clear();
        stdin().read_line(&mut input)?; // wait for next enter key press
        let command = input.trim();
        if "print".starts_with(command) {
            control_sender_tty.send(ControlMessage::Print).unwrap();
        }
        if "next".starts_with(command) {
            control_sender_tty.send(ControlMessage::NewRun).unwrap();
        }
        if "quit".starts_with(command) {
            stop_the_show = true;
        }
    }

    println!("Closing connection");
    Ok(())
}

#[derive(Parser)]
struct Cli {
    /// Name of the practice program to play
    practice_program: String,

    /// Midi device port (indexed by 0)
    #[arg(short, long, default_value_t = 0)]
    midi_device_port: usize,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match run(cli) {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}
