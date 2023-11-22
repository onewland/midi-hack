use std::{collections::HashMap, process::ExitStatus};

use lazy_static::lazy_static;
use log::info;

lazy_static! {
    static ref NOTE_PRONUNCIATIONS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("Bb", "B Flat");
        map.insert("Eb", "E Flat");
        map.insert("Ab", "A Flat");
        map
    };
    static ref INTERVAL_NAMES: HashMap<u8, &'static str> = {
        let mut map = HashMap::new();
        map.insert(0, "unison");
        map.insert(1, "half step");
        map.insert(2, "whole step");
        map.insert(3, "minor third");
        map.insert(4, "major third");
        map.insert(5, "perfect fourth");
        map.insert(6, "tritone");
        map.insert(7, "tonic");
        map.insert(8, "minor sixth");
        map.insert(9, "major sixth");
        map.insert(10, "minor seventh");
        map.insert(11, "major seventh");
        map.insert(12, "octave");
        map
    };
}

pub fn get_pronunciation(note: &str) -> &str {
    let special_pronunciation = NOTE_PRONUNCIATIONS.get(&note);
    return match special_pronunciation {
        Some(special) => *special,
        None => note,
    };
}

pub fn get_interval_name(interval: u8) -> &'static str {
    return match INTERVAL_NAMES.get(&interval) {
        Some(special) => *special,
        None => "unknown interval",
    };
}

pub fn say(text: String) -> ExitStatus {
    volume_say(text, Option::None)
}

pub fn volume_say(text: String, volume: Option<u8>) -> ExitStatus {
    let mut volume_prefixed_string =
        String::from(format!("[[volm {}]] ", volume.unwrap_or(50) as f32 / 100.0));
    volume_prefixed_string.push_str(&text);

    info!("attempting say: {}", volume_prefixed_string);

    std::process::Command::new("say")
        .arg("--voice=Moira")
        .arg(volume_prefixed_string)
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
}
