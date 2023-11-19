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

fn always_true(k: &&KeyMessage) -> bool {
    true
}

type FilterMethod = fn(&&KeyMessage) -> bool;

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
        return self.last_n_messages_reverse_chron(
            Some(|k: &&KeyMessage| k.message_type == MidiMessageTypes::NoteOff),
            n,
        );
    }

    pub fn last_n_key_downs_reversed(&self, n: usize) -> Vec<KeyMessage> {
        return self.last_n_messages_reverse_chron(
            Some(|k: &&KeyMessage| k.message_type == MidiMessageTypes::NoteOn),
            n,
        );
    }

    pub fn last_n_messages_reverse_chron(
        &self,
        custom_filter: Option<FilterMethod>,
        n: usize,
    ) -> Vec<KeyMessage> {
        return self
            .buf
            .read()
            .unwrap()
            .iter()
            .rev()
            .filter(custom_filter.unwrap_or(always_true))
            .take(n)
            .map(|k| *k)
            .collect::<Vec<KeyMessage>>();
    }
}
