pub mod midi {
    #[derive(Clone, Copy, Debug)]
    pub struct KeyMessage {
        pub timestamp: u64,
        pub message_type: MidiMessageTypes,
        pub key: u8,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum MidiMessageTypes {
        KeyDown = 144,
        KeyUp = 128,
        // KeepAlive = 254,
    }

    pub const KEY_DOWN: u8 = 144;
    pub const KEY_UP: u8 = 128;
    pub const KEEP_ALIVE: u8 = 254;
    const TIME_KEEPING: u8 = 208;
    pub static KNOWN_MESSAGE_TYPES: &'static [u8] = &[KEY_DOWN, KEY_UP, KEEP_ALIVE, TIME_KEEPING];

    // real pianos start with a low A, the midi standard starts at C
    const NOTE_SEQ_OFFSET: usize = 3;

    impl KeyMessage {
        pub fn readable_note(&self) -> String {
            return format!("{}{}", self.note_name(), self.key / 12);
        }

        pub fn note_name(&self) -> &str {
            let keys = [
                "A", "Bb", "B", "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab",
            ];
            keys.get((usize::from(self.key) + NOTE_SEQ_OFFSET) % keys.len())
                .unwrap()
        }

        pub fn print(&self) {
            print!("{:?}{} ", self.message_type, self.readable_note());
        }
    }

    pub fn build_key_message(timestamp: u64, unstructured_message: &[u8]) -> KeyMessage {
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
}

pub mod music {
    use log::trace;

    use crate::midi::{KeyMessage, MidiMessageTypes};

    pub fn is_minor_maj_7_chord(buf: &Vec<KeyMessage>) -> bool {
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

                let mut key_down_notes: Vec<&str> = key_downs
                    .get(start_run_index..end_run_idx)
                    .unwrap()
                    .iter()
                    .map(|m| m.note_name())
                    .collect();
                key_down_notes.sort();

                if key_down_notes == major_minor_chord_c {
                    return true;
                } else {
                    trace!(
                        "run indices = ({},{}), sorted_notes = {:?}, reference = {:?}",
                        start_run_index,
                        end_run_idx,
                        key_down_notes,
                        major_minor_chord_c
                    );
                }
                start_run_index += 1
            }
        }

        return false;
    }

    pub fn scale_matches_increments(key_events: &Vec<KeyMessage>, proper_deltas: [u8; 7]) -> bool {
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
}
