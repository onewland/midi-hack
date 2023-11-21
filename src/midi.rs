#[derive(Clone, Copy, Debug)]
pub struct KeyMessage {
    pub timestamp: u64, // TODO make this an option for user-generated messages
    pub message_type: MidiMessageTypes,
    pub key: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiMessageTypes {
    NoteOn = 144,
    NoteOff = 128,
    // KeepAlive = 254,
}

pub const KEY_DOWN: u8 = 144;
pub const KEY_UP: u8 = 128;
pub const KEEP_ALIVE: u8 = 254;
const TIME_KEEPING: u8 = 208;
pub static KNOWN_MESSAGE_TYPES: &'static [u8] = &[KEY_DOWN, KEY_UP, KEEP_ALIVE, TIME_KEEPING];
const VELOCITY: u8 = 0x64;

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

    pub fn to_string(&self) -> String {
        format!("{:?}{} ", self.message_type, self.readable_note())
    }

    pub fn encode(&self) -> [u8; 3] {
        return [self.message_type as u8, self.key, VELOCITY];
    }

    pub fn from_midi(timestamp: u64, unstructured_message: &[u8]) -> KeyMessage {
        let m_type = match unstructured_message[0] {
            KEY_DOWN => MidiMessageTypes::NoteOn,
            KEY_UP => MidiMessageTypes::NoteOff,
            _ => panic!("unknown message type"),
        };
        return KeyMessage {
            timestamp: timestamp,
            message_type: m_type,
            key: unstructured_message[1],
        };
    }
}
