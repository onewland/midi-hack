use std::{collections::BTreeMap, sync::RwLock};

use log::trace;

use crate::midi::{KeyMessage, MidiMessageTypes};

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

impl HoldStatus {
    pub fn down_like(self) -> bool {
        self == HoldStatus::PRESS || self == HoldStatus::DOWN
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeyStatus {
    pub key: u8,
    pub status: HoldStatus,
}

pub type TimeBucketedSparseKeyData = BTreeMap<u64, Vec<KeyStatus>>;

#[derive(Clone)]
pub struct HoldData {
    max_bucket_count: usize,
    buf: TimeBucketedSparseKeyData,
}

impl HoldData {
    pub fn new(bucket_count: usize) -> HoldData {
        HoldData {
            max_bucket_count: bucket_count,
            buf: BTreeMap::new(),
        }
    }

    pub fn print(&self) {
        print!("{:?}", self.buf);
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn raw_buf(&self) -> TimeBucketedSparseKeyData {
        return self.buf.clone();
    }

    pub fn update(&mut self, msg: KeyMessage) {
        let new_ts = crate::time::get_time();
        trace!("[holds_update] new_ts = {new_ts}");
        if let Some(last_seen) = self.buf.last_entry() {
            let old_ts = last_seen.key();
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
                            && (hold.status == HoldStatus::DOWN || hold.status == HoldStatus::PRESS)
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

            if new_ts != *old_ts {
                while self.buf.len() >= self.max_bucket_count {
                    self.buf.first_entry().unwrap().remove();
                }
            }
            self.buf.insert(new_ts, new_holds);
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
                self.buf.insert(new_ts, hold_container);
            }
        }
    }
}

pub struct KeyDb {
    ///
    /// Map of timestamp to hold data (this is filled in on-demand)
    ///
    holds: RwLock<HoldData>,
    linear_buf: RwLock<Vec<KeyMessage>>,
}

fn always_true(_k: &&KeyMessage) -> bool {
    true
}

type FilterMethod = fn(&&KeyMessage) -> bool;

impl KeyDb {
    pub fn new(bucket_count: usize) -> KeyDb {
        KeyDb {
            linear_buf: RwLock::from(Vec::new()),
            holds: RwLock::from(HoldData::new(bucket_count)),
        }
    }

    pub fn flat_message_log(&self) -> Vec<KeyMessage> {
        self.linear_buf.read().unwrap().to_vec()
    }

    pub fn print_holds(&self) {
        self.holds.read().unwrap().print()
    }

    pub fn push_msg(&self, msg: KeyMessage) {
        self.linear_buf.write().unwrap().push(msg);
        self.holds.try_write().unwrap().update(msg);
    }

    pub fn clear(&self) {
        self.linear_buf.write().unwrap().clear();
        self.holds.write().unwrap().clear();
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
        return self.holds.read().unwrap().raw_buf();
    }
}
