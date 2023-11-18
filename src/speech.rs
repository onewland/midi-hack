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
}

pub fn get_pronunciation(note: &str) -> &str {
    let special_pronunciation = NOTE_PRONUNCIATIONS.get(&note);
    return match special_pronunciation {
        Some(special) => *special,
        None => note,
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
