use std::{cmp::max, thread::JoinHandle};
use std::io::stdin;
// use std::process::Command;
use std::error::Error;
use std::sync::mpsc::{sync_channel, TryRecvError, RecvTimeoutError};
use std::sync::mpsc::Receiver;

use midir::{Ignore, MidiInput};

const KEY_DOWN: u8 = 144;
const KEY_UP: u8 = 128;
const KEEP_ALIVE: u8 = 254;
const TIME_KEEPING: u8 = 208;

const BREAK_DELAY_MICROSECONDS: u64 = 4_000_000;

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

impl KeyMessage {
    fn readable_note(&self) -> String {
        let keys = [
            "A", "Bb", "B", "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab",
        ];
        let str_note = keys
            .get((usize::from(self.key) + NOTE_SEQ_OFFSET) % keys.len())
            .unwrap();
        return format!("{}{}", str_note, self.key / 12);
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
    heartbeat_count: usize
}

impl KeyBuffer {
    fn new() -> KeyBuffer {
        return KeyBuffer {
            buf: Vec::new(),
            most_recent_insert: 0,
            run_end_listeners: Vec::new(),
            heartbeat_count: 0
        };
    }

    fn accept(&mut self, message: KeyMessage) {
        if message.timestamp - self.most_recent_insert > BREAK_DELAY_MICROSECONDS {
            self.end_run();
        }
        self.buf.push(message);
        self.most_recent_insert = max(message.timestamp, self.most_recent_insert);
        self.print();
    }

    fn end_run(&mut self) {
        println!("calling listeners");
        for listener in &self.run_end_listeners {
            listener.as_ref().on_run_end(&self.buf);
        }
        println!("new run");
        self.buf.clear();
        self.heartbeat_count = 0;
    }

    fn heartbeat(&mut self, elapsed: u64) {
        if self.heartbeat_count > 10 {
            self.end_run()
        }
        self.heartbeat_count += 1;
        self.print()
    }

    fn add_listener<T: 'static + RunEndListener + Send>(&mut self, listener: T) {
        self.run_end_listeners.push(Box::new(listener))
    }

    fn print(&self) {
        print!(
            "KeyBuffer [ most_recent_insert = {} ] [ keys = ",
            self.most_recent_insert
        );
        for msg in &self.buf {
            msg.print()
        }
        println!("]")
    }

    pub(crate) fn start_recv_loop(mut self, receiver: Receiver<KeyMessage>, heartbeat_receiver: Receiver<u64>) -> JoinHandle<()> {
        std::thread::spawn(move || {
            loop {
                match receiver.recv_timeout(std::time::Duration::from_nanos(100)) {
                    Ok(message) => self.accept(message),
                    Err(_recv_timeout_error) => (), // this is fine
                };
                match heartbeat_receiver.recv_timeout(std::time::Duration::from_nanos(100)) {
                    Ok(message) => self.heartbeat(message),
                    Err(_recv_timeout_error) => (), // this is fine
                }
            }
        })
    }
}

trait RunEndListener {
    fn on_run_end(&self, buf: &Vec<KeyMessage>);
}

struct AscendingScaleNotifier {}

impl RunEndListener for AscendingScaleNotifier {
    fn on_run_end(&self, buf: &Vec<KeyMessage>) {
        println!("is major scale: {}", is_ascending_major_scale(&buf));

        if is_ascending_major_scale(&buf) {
            sentry::capture_message(
                format!(
                    "user played major scale starting at {}",
                    buf[0].readable_note()
                )
                .as_str(),
                sentry::Level::Info,
            );
        }
    }
}

fn is_ascending_major_scale(key_events: &Vec<KeyMessage>) -> bool {
    // there should be [multiple of] 16 key-down then up events,
    // for 8 notes played and then lifted
    if key_events.len() < 16 || key_events.len() % 16 != 0 {
        return false;
    }

    // an ascending major scale is the following sequence of key
    // down followed by up of the same note with no overlap.
    //
    // whole [step] - whole - half - whole - whole - whole - half
    let proper_deltas = vec![2, 2, 1, 2, 2, 2, 1];
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
        if pair_based_index > 0
            && e1.key > base_note
            && (e1.key - base_note != proper_deltas[(pair_based_index % 8) - 1])
        {
            return false;
        } else {
            // this pair is good, on to the next one and update the base note
            base_note = e1.key
        }
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
    let (midi_sender, midi_receiver) = sync_channel(1);
    let (heartbeat_sender, heartbeat_receiver) = sync_channel(1);

    let known_message_types = vec![KEY_DOWN, KEY_UP, KEEP_ALIVE, TIME_KEEPING];
    let mut buf = KeyBuffer::new();
    buf.add_listener(AscendingScaleNotifier {});

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
                    midi_sender.send(parsed_message);
                    // buf.accept(parsed_message);
                    // buf.print();
                }
            }
        },
        (),
    )?;

    std::thread::spawn(move || {
        const HEARTBEAT_LAPSE_SECONDS : u64 = 1;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(HEARTBEAT_LAPSE_SECONDS));
            heartbeat_sender.send(HEARTBEAT_LAPSE_SECONDS);
        }
    });

    buf.start_recv_loop(midi_receiver, heartbeat_receiver);

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");
    Ok(())
}

fn main() {
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
