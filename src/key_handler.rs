use std::sync::RwLock;

use crate::midi::{KeyMessage, MidiMessageTypes};

pub enum ControlMessage {
    Heartbeat,
    NewRun,
    Print,
}

pub struct KeyDb {
    buf: RwLock<Vec<KeyMessage>>,
}

impl KeyDb {
    pub fn new() -> KeyDb {
        KeyDb {
            buf: RwLock::from(Vec::new()),
        }
    }

    pub fn flat_message_log(&self) -> Vec<KeyMessage> {
        self.buf.read().unwrap().to_vec()
    }

    pub fn push_msg(&self, key: KeyMessage) {
        self.buf.write().unwrap().push(key)
    }

    pub fn clear(&self) {
        self.buf.write().unwrap().clear()
    }

    pub fn last_n_key_ups_reversed(&self, n: usize) -> Vec<KeyMessage> {
        return self.buf
            .read()
            .unwrap()
            .iter()
            .rev()
            .filter(|k| k.message_type == MidiMessageTypes::KeyUp)
            .take(n)
            .map(|k| *k)
            .collect::<Vec<KeyMessage>>();
    }
}
