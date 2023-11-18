use std::{
    sync::mpsc::SyncSender,
    sync::{mpsc::Receiver, Arc},
};

use log::{info, trace};

use crate::{
    key_handler::{ControlMessage, KeyDb},
    midi::{C4_DOWN, F4_DOWN, F4_UP},
};
use crate::{midi::KeyMessage, speech::say};
use crate::{midi::C4_UP, speech::get_pronunciation};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticeProgramState {
    INITIALIZING,
    LISTENING,
    PROMPTING,
    FINISHED,
}

pub trait PracticeProgram {
    fn get_state(&self) -> PracticeProgramState;

    fn run(self);
}

pub struct FreePlayPracticeProgram {
    state: PracticeProgramState,
    ctrl_sender: SyncSender<ControlMessage>,
    key_receiver: Receiver<KeyMessage>,
    key_db: Arc<KeyDb>,
}

impl PracticeProgram for FreePlayPracticeProgram {
    fn get_state(&self) -> PracticeProgramState {
        return self.state;
    }

    fn run(mut self) {
        info!("starting FreePlayPracticeProgram");
        self.state = PracticeProgramState::LISTENING;
        std::thread::spawn(move || loop {
            let msg = self.key_receiver.recv().unwrap();
            self.on_keypress(msg);
        });
    }
}

impl FreePlayPracticeProgram {
    pub fn new(
        ctrl_sender: SyncSender<ControlMessage>,
        key_receiver: Receiver<KeyMessage>,
        key_db: Arc<KeyDb>,
    ) -> FreePlayPracticeProgram {
        FreePlayPracticeProgram {
            state: PracticeProgramState::INITIALIZING,
            ctrl_sender,
            key_receiver,
            key_db,
        }
    }

    fn on_keypress(&self, latest: KeyMessage) {
        log::trace!("received KeyMessage {}", latest.to_string());
        let kmsg_log = self.key_db.flat_message_log();
        let major_scale_deltas = [2, 2, 1, 2, 2, 2, 1];
        let major_scale_up_and_down_deltas = [2, 2, 1, 2, 2, 2, 1, -1, -2, -2, -2, -1, -2, -2];

        let harmonic_minor_scale_deltas = [2, 1, 2, 2, 1, 3, 1];

        let reverse_chron_key_events = &self.key_db.last_n_key_ups_reversed(15);
        trace!("reverse_chron_key_events = {:?}", reverse_chron_key_events);
        if reverse_chron_key_events.len() > 7 {
            if let Some(msg) =
                crate::music::detect_run(&reverse_chron_key_events[0..8], &major_scale_deltas)
            {
                log::info!(
                    "user played ascending section of major scale starting at {}",
                    msg.note_name()
                );
            }
            if let Some(msg) = crate::music::detect_run(
                &reverse_chron_key_events[0..8],
                &harmonic_minor_scale_deltas,
            ) {
                log::info!(
                    "user played harmonic minor scale starting at {}",
                    msg.note_name()
                );
                self.ctrl_sender.send(ControlMessage::NewRun).unwrap();
            }
        }
        if reverse_chron_key_events.len() > 14 {
            if let Some(msg) = crate::music::detect_run(
                &reverse_chron_key_events[0..15],
                &major_scale_up_and_down_deltas,
            ) {
                log::info!(
                    "user played up-and-down major scale starting at {}",
                    msg.note_name()
                );
                self.ctrl_sender.send(ControlMessage::NewRun).unwrap();
            }
        }

        let result = crate::music::is_minor_maj_7_chord(&kmsg_log);
        if result {
            log::info!(
                "user played minor-maj7 chord starting at {}",
                kmsg_log[0].readable_note()
            );
            self.ctrl_sender.send(ControlMessage::NewRun).unwrap();
        }
    }
}

pub struct CircleOfFourthsPracticeProgram {
    state: PracticeProgramState,
    ctrl_sender: SyncSender<ControlMessage>,
    key_receiver: Receiver<KeyMessage>,
    key_db: Arc<KeyDb>,
    current_key: usize,
}

const KEYS_IN_CIRCLE_OF_FOURTHS_ORDER: &'static [&'static str] = &[
    "C", "F", "Bb", "Eb", "Ab", "C#", "F#", "B", "E", "A", "D", "G",
];

impl CircleOfFourthsPracticeProgram {
    pub fn new(
        ctrl_sender: SyncSender<ControlMessage>,
        key_receiver: Receiver<KeyMessage>,
        key_db: Arc<KeyDb>,
    ) -> CircleOfFourthsPracticeProgram {
        CircleOfFourthsPracticeProgram {
            state: PracticeProgramState::INITIALIZING,
            ctrl_sender,
            key_receiver,
            key_db,
            current_key: 0,
        }
    }

    fn request_current_key(&mut self) {
        if self.state != PracticeProgramState::FINISHED {
            self.state = PracticeProgramState::PROMPTING;
            say(format!(
                "play {} mayjur",
                get_pronunciation(KEYS_IN_CIRCLE_OF_FOURTHS_ORDER[self.current_key])
            ));
            self.state = PracticeProgramState::LISTENING;
        }
    }

    fn advance_current_key(&mut self) {
        if self.current_key + 1 < KEYS_IN_CIRCLE_OF_FOURTHS_ORDER.len() {
            self.current_key += 1;
        } else {
            say("you've finished the program. good job!".into());
            self.state = PracticeProgramState::FINISHED;
        }
    }

    fn on_keypress(&mut self, _latest: KeyMessage) {
        if self.state == PracticeProgramState::FINISHED {
            return;
        }

        let reverse_chron_key_events = &self.key_db.last_n_key_ups_reversed(15);
        let major_scale_up_and_down_deltas = [2, 2, 1, 2, 2, 2, 1, -1, -2, -2, -2, -1, -2, -2];

        if reverse_chron_key_events.len() > 14 {
            if let Some(msg) =
                crate::music::detect_run(&reverse_chron_key_events, &major_scale_up_and_down_deltas)
            {
                log::info!(
                    "user played major scale starting at {}",
                    msg.readable_note()
                );
                self.ctrl_sender.send(ControlMessage::NewRun).unwrap();

                if msg.note_name() == KEYS_IN_CIRCLE_OF_FOURTHS_ORDER[self.current_key] {
                    self.advance_current_key();
                    self.request_current_key();
                } else {
                    say("You've played a major scale but in the wrong key.".into());
                    self.request_current_key();
                }
            }
        }
    }
}

impl PracticeProgram for CircleOfFourthsPracticeProgram {
    fn get_state(&self) -> PracticeProgramState {
        return self.state;
    }

    fn run(mut self) {
        info!("starting CircleOfFourthsPracticeProgram");
        self.request_current_key();
        self.state = PracticeProgramState::LISTENING;
        std::thread::spawn(move || loop {
            let msg = self.key_receiver.recv().unwrap();
            self.on_keypress(msg);
        });
    }
}

pub struct EarTrainingPracticeProgram {
    state: PracticeProgramState,
    ctrl_sender: SyncSender<ControlMessage>,
    midi_out_sender: SyncSender<KeyMessage>,
    key_receiver: Receiver<KeyMessage>,
    key_db: Arc<KeyDb>,
}

impl EarTrainingPracticeProgram {
    pub fn new(
        ctrl_sender: SyncSender<ControlMessage>,
        midi_out_sender: SyncSender<KeyMessage>,
        key_receiver: Receiver<KeyMessage>,
        key_db: Arc<KeyDb>,
    ) -> EarTrainingPracticeProgram {
        EarTrainingPracticeProgram {
            state: PracticeProgramState::INITIALIZING,
            midi_out_sender,
            ctrl_sender,
            key_receiver,
            key_db,
        }
    }

    fn on_keypress(&mut self, _latest: KeyMessage) {
        if self.state == PracticeProgramState::FINISHED {
            return;
        }
    }
}

impl PracticeProgram for EarTrainingPracticeProgram {
    fn get_state(&self) -> PracticeProgramState {
        return self.state;
    }

    fn run(mut self) {
        info!("starting EarTrainingPracticeProgram");
        self.state = PracticeProgramState::LISTENING;

        std::thread::spawn(move || {
            say("starting ear training".into());

            // await channel readiness
            loop {
                match self.midi_out_sender.try_send(C4_DOWN) {
                    Ok(_) => break,
                    Err(_) => std::thread::sleep(std::time::Duration::from_nanos(100)),
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
            self.midi_out_sender.send(C4_UP).unwrap();
            self.midi_out_sender.send(F4_DOWN).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1000));
            self.midi_out_sender.send(F4_UP).unwrap();
            loop {
                let msg = self.key_receiver.recv().unwrap();
                self.on_keypress(msg);
            }
        });
    }
}
