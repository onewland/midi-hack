use std::sync::RwLock;

use crate::midi::{KeyMessage, MidiMessageTypes};

pub enum ControlMessage {
    Heartbeat,
    NewRun,
    Print,
}

pub struct KeyDb {
    holds: RwLock<Vec<Vec<u8>>>,
    linear_buf: RwLock<Vec<KeyMessage>>,
    base_time: u8,
}

fn always_true(_k: &&KeyMessage) -> bool {
    true
}

type FilterMethod = fn(&&KeyMessage) -> bool;

impl KeyDb {
    pub fn new(bucket_count: usize) -> KeyDb {
        KeyDb {
            linear_buf: RwLock::from(Vec::new()),
            holds: RwLock::from(Vec::with_capacity(bucket_count)),
            base_time: 0,
        }
    }

    pub fn flat_message_log(&self) -> Vec<KeyMessage> {
        self.linear_buf.read().unwrap().to_vec()
    }

    pub fn push_msg(&self, key: KeyMessage) {
        self.linear_buf.write().unwrap().push(key)
    }

    pub fn clear(&self) {
        self.linear_buf.write().unwrap().clear()
    }

    fn holds_update(&self, msg: KeyMessage) {}

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
            .linear_buf
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
