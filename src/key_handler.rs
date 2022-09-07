use std::sync::RwLock;

use crate::midi::KeyMessage;

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

    pub fn ro_buffer(&self) -> Vec<KeyMessage> {
        self.buf.read().unwrap().to_vec()
    }

    pub fn push_msg(&self, key: KeyMessage) {
        self.buf.write().unwrap().push(key)
    }

    pub fn clear(&self) {
        self.buf.write().unwrap().clear()
    }
}