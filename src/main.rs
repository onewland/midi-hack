use std::io::stdin;
use std::{cmp::max, thread::JoinHandle};
// use std::process::Command;
use std::error::Error;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::Receiver;

use log::{debug, info, trace};

use midir::{Ignore, MidiInput};

const KEY_DOWN: u8 = 144;
const KEY_UP: u8 = 128;
const KEEP_ALIVE: u8 = 254;
const TIME_KEEPING: u8 = 208;

const HEARTBEATS_PER_AUTO_NEW_RUN: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
enum MidiMessageTypes {
    KeyDown = 144,
    KeyUp = 128,
    KeepAlive = 254,
}

#[derive(Clone, Copy, Debug)]
struct KeyMessage {
    timestamp: u64,
    message_type: MidiMessageTypes,
    key: u8,
}

enum ControlMessage {
    Heartbeat,
    NewRun,
    Print,
}

impl KeyMessage {
    fn readable_note(&self) -> String {
        return format!("{}{}", self.note_name(), self.key / 12);
    }

    fn note_name(&self) -> &str {
        let keys = [
            "A", "Bb", "B", "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab",
        ];
        keys.get((usize::from(self.key) + NOTE_SEQ_OFFSET) % keys.len())
            .unwrap()
    }

    fn print(&self) {
        print!("{:?}{} ", self.message_type, self.readable_note());
    }
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
        self.call_listeners();
    }

    fn end_run(&mut self) {
        info!("{}", "[new run]");
        self.buf.clear();
        self.heartbeat_count = 0;
        self.print()
    }

    fn call_listeners(&mut self) {
        let mut hit_end = false;
        for listener in &self.run_end_listeners {
            hit_end = hit_end || listener.as_ref().on_keypress(&self.buf);
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
    fn on_keypress(&self, buf: &Vec<KeyMessage>) -> bool;
}

struct AscendingScaleNotifier;
impl RunEndListener for AscendingScaleNotifier {
    fn on_keypress(&self, buf: &Vec<KeyMessage>) -> bool {
        let major_scale_deltas = [2, 2, 1, 2, 2, 2, 1];
        let harmonic_minor_scale_deltas = [2, 1, 2, 2, 1, 3, 1];

        if scale_matches_increments(&buf, major_scale_deltas) {
            sentry::capture_message(
                format!(
                    "user played major scale starting at {}",
                    buf[0].readable_note()
                )
                .as_str(),
                sentry::Level::Info,
            );
            return true;
        }

        if scale_matches_increments(&buf, harmonic_minor_scale_deltas) {
            sentry::capture_message(
                format!(
                    "user played harmonic minor scale starting at {}",
                    buf[0].readable_note()
                )
                .as_str(),
                sentry::Level::Info,
            );
            return true;
        }

        return false;
    }
}

fn is_minor_maj_7_chord(buf: &Vec<KeyMessage>) -> bool {
    let mut major_minor_chord_c = vec!["C", "Eb", "G", "B"];
    major_minor_chord_c.sort();

    if buf.len() < 8 {
        return false;
    }

    let key_downs: Vec<&KeyMessage> = buf
        .iter()
        .filter(|m| m.message_type == MidiMessageTypes::KeyDown)
        .collect();

    let timestamp_threshold = 30000;

    if key_downs.len() >= 4 {
        let mut start_run_index = 0;
        while start_run_index < key_downs.len() {
            let mut end_run_idx = start_run_index + 1;

            while end_run_idx < key_downs.len()
                && key_downs[end_run_idx].timestamp - key_downs[start_run_index].timestamp
                    < timestamp_threshold
            {
                end_run_idx += 1;
            }

            let mut key_downs: Vec<&str> = key_downs
                .get(start_run_index..end_run_idx)
                .unwrap()
                .iter()
                .map(|m| m.note_name())
                .collect();
            key_downs.sort();

            if key_downs == major_minor_chord_c {
                return true;
            } else {
                trace!(
                    "run indices = ({},{}), sorted_notes = {:?}, reference = {:?}",
                    start_run_index, end_run_idx, key_downs, major_minor_chord_c
                );
            }
            start_run_index += 1
        }
    }

    return false;

}

struct MinorMajor7ChordListener;
impl RunEndListener for MinorMajor7ChordListener {
    fn on_keypress(&self, buf: &Vec<KeyMessage>) -> bool {
        let result =  is_minor_maj_7_chord(buf);
        if result {
            sentry::capture_message(
                format!(
                    "user played minor-maj7 chord starting at {}",
                    buf[0].readable_note()
                )
                .as_str(),
                sentry::Level::Info,
            );
        }
        return result
    }
}

fn scale_matches_increments(key_events: &Vec<KeyMessage>, proper_deltas: [u8; 7]) -> bool {
    // there should be [multiple of] 16 key-down then up events,
    // for 8 notes played and then lifted
    if key_events.len() < 16 || key_events.len() % 16 != 0 {
        return false;
    }

    // an ascending major scale is the following sequence of key
    // down followed by up of the same note with no overlap.
    //
    // whole [step] - whole - half - whole - whole - whole - half
    let mut pair_based_index = 0;
    let mut base_note = key_events[0].key;

    // go pair by pair
    while pair_based_index < key_events.len() / 2 {
        let e1 = key_events[pair_based_index * 2];
        let e2 = key_events[pair_based_index * 2 + 1];
        // enforce down then up
        if e1.message_type != MidiMessageTypes::KeyDown
            || e2.message_type != MidiMessageTypes::KeyUp
        {
            return false;
        }

        // enforce same key down then up
        if e1.key != e2.key {
            return false;
        }

        // if not on the first pair, make sure we're moving up, and by correct number of steps
        if pair_based_index > 0 {
            if e1.key <= base_note {
                return false;
            }
            if e1.key - base_note != proper_deltas[(pair_based_index % 8) - 1] {
                return false;
            }
        }

        // nothing eliminated this pair, updated the base note and move on
        base_note = e1.key;
        pair_based_index += 1
    }

    return true;
}

// real pianos start with a low A, the midi standard starts at C
const NOTE_SEQ_OFFSET: usize = 3;

fn build_key_message(timestamp: u64, unstructured_message: &[u8]) -> KeyMessage {
    let m_type = match unstructured_message[0] {
        KEY_DOWN => MidiMessageTypes::KeyDown,
        KEY_UP => MidiMessageTypes::KeyUp,
        _ => panic!("unknown message type"),
    };
    return KeyMessage {
        timestamp: timestamp,
        message_type: m_type,
        key: unstructured_message[1],
    };
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

    let known_message_types = vec![KEY_DOWN, KEY_UP, KEEP_ALIVE, TIME_KEEPING];
    let mut buf = KeyBuffer::new();
    buf.add_listener(AscendingScaleNotifier {});
    buf.add_listener(MinorMajor7ChordListener {});

    // Start the read loop
    let _conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |stamp, message, _| {
            if !known_message_types.contains(&message[0]) {
                println!(
                    "unknown message {}: {:?} (len = {})",
                    stamp,
                    message,
                    message.len()
                );
            }
            if message.len() == 3 {
                if message[0] == KEY_UP || message[0] == KEY_DOWN {
                    let parsed_message = build_key_message(stamp, message);
                    playback_sender.send(parsed_message);
                }
            }
        },
        (),
    )?;

    std::thread::spawn(move || {
        const HEARTBEAT_LAPSE_SECONDS: u64 = 1;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(HEARTBEAT_LAPSE_SECONDS));
            control_sender.send(ControlMessage::Heartbeat);
        }
    });
    buf.start_recv_loop(playback_receiver, control_receiver);

    let mut stop_the_show = false;

    while !stop_the_show {
        input.clear();
        stdin().read_line(&mut input)?; // wait for next enter key press
        let command = input.trim();
        if "print".starts_with(command) {
            control_sender_fg.send(ControlMessage::Print);
        }
        if "next".starts_with(command) {
            control_sender_fg.send(ControlMessage::NewRun);
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
    let _guard = sentry::init((
        "https://29e00247e7b64440822c2be63f3baa0f@o1066102.ingest.sentry.io/6678046",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    // let _test = Command::new("say")
    //     .arg("--rate=250")
    //     .arg("rust program talking")
    //     .output();
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}
