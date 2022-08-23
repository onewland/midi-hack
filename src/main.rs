use std::io::stdin;
// use std::process::Command;
use std::{error::Error};

use midir::{Ignore, MidiInput};

const KEY_DOWN: u8 = 144;
const KEY_UP: u8 = 128;
const KEEP_ALIVE: u8 = 254;
const TIME_KEEPING: u8 = 208;

#[derive(Debug)]
enum MidiMessageTypes {
    KeyDown = 144,
    KeyUp = 128,
    KeepAlive = 254,
}

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
        return format!("{}{}", str_note, self.key/12);
    }
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
    let mut input = String::new();
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => panic!("no device found"),
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            panic!("don't know how to deal with multiple devices")
        }
    };
    println!("\nOpening connection");
    let _in_port_name = midi_in.port_name(in_port)?;
    let known_message_types = vec![KEY_DOWN, KEY_UP, KEEP_ALIVE, TIME_KEEPING];

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
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
                    println!(
                        "{:?} {:?}: {}",
                        parsed_message.timestamp,
                        parsed_message.message_type,
                        parsed_message.readable_note()
                    );
                    // sentry::capture_message(format!("user played {str_note}").as_str(), sentry::Level::Info);
                }
            }
        },
        (),
    )?;
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
