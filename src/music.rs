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

///
/// run_matches_increments takes a key run, *in reverse chronological order*, and
/// returns true if the difference between keypresses passed in `in_order_increments`
/// are encountered in reverse order. Chords are not accounted for -- notes are
/// assumed to be played independently
///
/// It returns None if the run doesn't match in_order_increments.
/// It returns the root key (as u8) if the run does match
///
/// The key_events passed here should only be key_ups. It may make sense
/// to change the parameter to take u8 notes.
///
/// The reverse order is used because upstream logic only wants to distinguish
/// whether the most "recent" run in the key database matches (ignoring earlier
/// mistakes/incidental key presses)
pub fn detect_run(
    reverse_chron_key_events: &[KeyMessage],
    in_order_increments: &[i8],
) -> Option<KeyMessage> {
    trace!(
        "detect_run({:?},{:?})",
        reverse_chron_key_events,
        in_order_increments
    );
    if reverse_chron_key_events.len() != in_order_increments.len() + 1 {
        panic!(
            "bad parameters passed to detect_run: {} events and {} increments (increments must be 1 less long than events)",
            reverse_chron_key_events.len(),
            in_order_increments.len()
        )
    }
    let incs_len = in_order_increments.len();
    let mut last_event = reverse_chron_key_events[0];
    let mut i = 1;
    while i < reverse_chron_key_events.len() {
        let current_event = reverse_chron_key_events[i];
        // TODO: this is dumb but I didn't want to add abs() or </> logic to handle unsigned
        // int overflow
        if i16::from(last_event.key) - i16::from(current_event.key)
            != in_order_increments[incs_len - i].into()
        {
            trace!(
                "current event = {:?}, last event = {:?}, in_order_increment tested = {}",
                current_event,
                last_event,
                in_order_increments[incs_len - 1]
            );
            return None;
        }
        last_event = current_event;
        i += 1;
    }
    return Some(last_event);
}