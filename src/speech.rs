use std::{collections::HashMap, process::ExitStatus};

use lazy_static::lazy_static;

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
    std::process::Command::new("say")
        .arg("--voice=Moira")
        .arg(text)
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
}
