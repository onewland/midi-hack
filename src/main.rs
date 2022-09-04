use std::io::stdin;
use std::{cmp::max, thread::JoinHandle};
// use std::process::Command;
use std::error::Error;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::Receiver;

use log::info;

use midi_hack::midi::{build_key_message, KeyMessage, KNOWN_MESSAGE_TYPES};
use midi_hack::music::{is_minor_maj_7_chord, scale_matches_increments};
use midir::{Ignore, MidiInput};

const HEARTBEATS_PER_AUTO_NEW_RUN: usize = 100;

enum ControlMessage {
    Heartbeat,
    NewRun,
    Print,
}

// #[derive(Clone)]
struct KeyBuffer {
    buf: Vec<KeyMessage>,
    most_recent_insert: u64,
    run_end_listeners: Vec<Box<dyn RunEndListener + Send>>,
    heartbeat_count: usize,
}

impl KeyBuffer {
    fn new() -> KeyBuffer {
        return KeyBuffer {
            buf: Vec::new(),
            most_recent_insert: 0,
            run_end_listeners: Vec::new(),
            heartbeat_count: 0,
        };
    }

    fn accept(&mut self, message: KeyMessage) {
        self.buf.push(message);
        self.most_recent_insert = max(message.timestamp, self.most_recent_insert);
        self.call_listeners(message);
    }

    fn end_run(&mut self) {
        info!("{}", "[new run]");
        self.buf.clear();
        self.heartbeat_count = 0;
        self.print()
    }

    fn call_listeners(&mut self, message: KeyMessage) {
        let mut hit_end = false;
        for listener in &self.run_end_listeners {
            hit_end = hit_end || listener.as_ref().on_keypress(&self.buf, message);
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

    fn add_listener<T: 'static + RunEndListener + Send>(&mut self, listener: T) {
        self.run_end_listeners.push(Box::new(listener))
    }

    fn print(&self) {
        print!(
            "KeyBuffer [ most_recent_insert = {} ] [ keys = ",
            self.most_recent_insert
        );
        let mut last_msg: Option<KeyMessage> = None;
        self.buf.iter().for_each(|msg| {
            // print rest time since prior note
            match last_msg {
                None => (),
                Some(prev) => print!("{} ", msg.timestamp - prev.timestamp),
            }
            // print note
            msg.print();

            last_msg = Some(*msg);
        });
        println!("]")
    }

    pub(crate) fn start_recv_loop(
        mut self,
        playback_receiver: Receiver<KeyMessage>,
        control_receiver: Receiver<ControlMessage>,
    ) -> JoinHandle<()> {
        std::thread::spawn(move || {
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
        })
    }
}

trait RunEndListener {
    // RunEndListener listens on runs for the end, if it returns
    // true it has detected an end of a run, false means that it has not
    fn on_keypress(&self, key_log: &Vec<KeyMessage>, latest: KeyMessage) -> bool;
}

struct AscendingScaleNotifier;
impl RunEndListener for AscendingScaleNotifier {
    fn on_keypress(&self, key_log: &Vec<KeyMessage>, latest: KeyMessage) -> bool {
        let major_scale_deltas = [2, 2, 1, 2, 2, 2, 1];
        let harmonic_minor_scale_deltas = [2, 1, 2, 2, 1, 3, 1];

        if scale_matches_increments(&key_log, major_scale_deltas) {
            log::info!(
                "user played major scale starting at {}",
                key_log[0].readable_note()
            );
            return true;
        }

        if scale_matches_increments(&key_log, harmonic_minor_scale_deltas) {
            log::info!(
                "user played harmonic minor scale starting at {}",
                key_log[0].readable_note()
            );
            return true;
        }

        return false;
    }
}

struct MinorMajor7ChordListener {
    // currently_pressed_keys: [(bool, u64)],
}
impl RunEndListener for MinorMajor7ChordListener {
    fn on_keypress(&self, key_log: &Vec<KeyMessage>, latest: KeyMessage) -> bool {
        let result = is_minor_maj_7_chord(key_log);
        if result {
            log::info!(
                "user played minor-maj7 chord starting at {}",
                key_log[0].readable_note()
            );
        }
        return result;
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Midi read setup
    let mut input = String::new();
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => panic!("no device found"),
        1 => {
            let device_name = midi_in.port_name(&in_ports[0]).unwrap();
            println!("Choosing the only available input port: {}", device_name);
            sentry::configure_scope(|scope| scope.set_tag("midi_device", device_name));
            &in_ports[0]
        }
        _ => {
            panic!("don't know how to deal with multiple devices")
        }
    };
    println!("\nOpening connection");
    let _in_port_name = midi_in.port_name(in_port)?;

    // Listener setup
    let (playback_sender, playback_receiver) = sync_channel(1);
    let (control_sender, control_receiver) = sync_channel(1);
    let control_sender_fg = control_sender.clone();

    let mut buf = KeyBuffer::new();
    buf.add_listener(AscendingScaleNotifier {});
    buf.add_listener(MinorMajor7ChordListener {});

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
                    let parsed_message = build_key_message(stamp, message);
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
    buf.start_recv_loop(playback_receiver, control_receiver);

    let mut stop_the_show = false;

    while !stop_the_show {
        input.clear();
        stdin().read_line(&mut input)?; // wait for next enter key press
        let command = input.trim();
        if "print".starts_with(command) {
            control_sender_fg.send(ControlMessage::Print).unwrap();
        }
        if "next".starts_with(command) {
            control_sender_fg.send(ControlMessage::NewRun).unwrap();
        }
        if "quit".starts_with(command) {
            stop_the_show = true;
        }
    }

    println!("Closing connection");
    Ok(())
}

fn main() {
    env_logger::init();
    // let _guard = sentry::init((
    //     "https://29e00247e7b64440822c2be63f3baa0f@o1066102.ingest.sentry.io/6678046",
    //     sentry::ClientOptions {
    //         release: sentry::release_name!(),
    //         ..Default::default()
    //     },
    // ));

    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}
