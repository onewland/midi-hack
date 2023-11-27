use std::collections::HashSet;

use log::trace;

use crate::key_handler::TimeBucketedSparseKeyData;
use crate::midi::KeyMessage;

/// returns true if a major-minor seven chord rooted by root_key is in the most recent interval
/// (allowing for the possiblity of other notes to be played simultaneously without returning false)
pub fn is_minor_maj_7_chord_in_holds(buf: &TimeBucketedSparseKeyData, root_key: u8) -> bool {
    let relevant_keys = HashSet::from([root_key, root_key + 3, root_key + 7, root_key + 11]);

    return all_keys_down_others_allowed(buf, relevant_keys);
}

/// returns true if all keys are down in the most recent interval
/// (allowing for the possiblity of other notes to be played simultaneously without returning false)
fn all_keys_down_others_allowed(
    buf: &TimeBucketedSparseKeyData,
    relevant_keys: HashSet<u8>,
) -> bool {
    if let Some((_ts, last_view)) = buf.last_key_value() {
        if last_view.len() < relevant_keys.len() {
            return false;
        }

        trace!("all_keys_down_others_allowed last_view: {:?}", last_view);
        let key_states = last_view
            .iter()
            .filter(|status| status.status.down_like() && relevant_keys.contains(&status.key));

        return Vec::from_iter(key_states).len() == relevant_keys.len();
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
    // trace!(
    //     "detect_run({:?},{:?})",
    //     reverse_chron_key_events,
    //     in_order_increments
    // );
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
