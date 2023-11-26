use std::{collections::BTreeMap, sync::RwLock};

use log::trace;

use crate::{
    midi::{KeyMessage, MidiMessageTypes},
    time::get_time,
};

pub enum ControlMessage {
    Heartbeat,
    NewRun,
    Print,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum HoldStatus {
    EMPTY,
    PRESS,
    DOWN,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyStatus {
    pub key: u8,
    pub status: HoldStatus,
}

pub type TimeBucketedSparseKeyData = BTreeMap<u64, Vec<KeyStatus>>;

pub struct KeyDb {
    ///
    /// Map of timestamp to hold data (this is filled in on-demand)
    ///
    holds: RwLock<TimeBucketedSparseKeyData>,
    linear_buf: RwLock<Vec<KeyMessage>>,
    base_time: u64,
}

fn always_true(_k: &&KeyMessage) -> bool {
    true
}

type FilterMethod = fn(&&KeyMessage) -> bool;

impl KeyDb {
    pub fn new(bucket_count: usize) -> KeyDb {
        KeyDb {
            linear_buf: RwLock::from(Vec::new()),
            holds: RwLock::from(BTreeMap::new()),
            base_time: get_time(),
        }
    }

    pub fn flat_message_log(&self) -> Vec<KeyMessage> {
        self.linear_buf.read().unwrap().to_vec()
    }

    pub fn print_holds(&self) {
        print!("{:?}", self.holds.read().unwrap());
    }

    pub fn push_msg(&self, key: KeyMessage) {
        self.linear_buf.write().unwrap().push(key);
        self.holds_update(key);
    }

    pub fn clear(&self) {
        self.linear_buf.write().unwrap().clear()
    }

    fn holds_update(&self, msg: KeyMessage) {
        match self.holds.try_write() {
            Ok(mut holds) => {
                let new_ts = crate::time::get_time();
                trace!("[holds_update] new_ts = {new_ts}");
                if let Some(last_seen) = holds.last_entry() {
                    let old_holds = last_seen.get();
                    // conditions to consider:
                    let mut new_holds = Vec::from_iter(
                        old_holds
                            .iter()
                            .filter(|hold| hold.status != HoldStatus::EMPTY) // old EMPTY messages are just noise after an interval
                            .map(|hold| {
                                // a key has been down in the last seen timestamp, with note off (end the hold)
                                if msg.message_type == MidiMessageTypes::NoteOff
                                    && hold.key == msg.key
                                    && (hold.status == HoldStatus::DOWN
                                        || hold.status == HoldStatus::PRESS)
                                {
                                    KeyStatus {
                                        key: msg.key,
                                        status: HoldStatus::EMPTY,
                                    }
                                } else if hold.status == HoldStatus::PRESS {
                                    // a key had PRESS in the last timestamp, transition to DOWN
                                    KeyStatus {
                                        key: hold.key,
                                        status: HoldStatus::DOWN,
                                    }
                                } else {
                                    // - a key has been down in the last seen timestamp, with no note off (continue the hold)
                                    *hold
                                }
                            }),
                    );

                    if msg.message_type == MidiMessageTypes::NoteOn {
                        new_holds.push(KeyStatus {
                            key: msg.key,
                            status: HoldStatus::PRESS,
                        })
                    }

                    holds.insert(new_ts, new_holds);
                }
                // no holds exist
                else {
                    if msg.message_type == MidiMessageTypes::NoteOn {
                        let new_hold = KeyStatus {
                            key: msg.key,
                            status: HoldStatus::PRESS,
                        };
                        let hold_container = Vec::from([new_hold]);
                        trace!("built hold struct {:?}", hold_container);
                        holds.insert(new_ts, hold_container);
                    }
                }
            }
            Err(_) => todo!(),
        }
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

    pub fn get_hold_data(&self) -> TimeBucketedSparseKeyData {
        return self.holds.read().unwrap().to_owned();
    }
}
