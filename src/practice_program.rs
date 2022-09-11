use std::{
    sync::mpsc::SyncSender,
    sync::{mpsc::Receiver, Arc},
};

use log::{info, trace};

use crate::{key_handler::ControlMessage, key_handler::KeyDb, midi::KeyMessage};

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

const KEYS_IN_CIRCLE_OF_FOURTHS_ORDER : &'static [&'static str] = &[
    "C",
    "F",
    "Bb",
    "Eb",
    "Ab",
    "C#",
    "F#",
    "B",
    "E",
    "A",
    "D",
    "G",
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

    fn say(&self, text: String) {
        let spawn = std::process::Command::new("say")
            .arg("--voice=Moira")
            .arg(text)
            .spawn();
        match spawn {
            Ok(mut child) => child.wait().unwrap(),
            Err(err) => panic!("{}", err),
        };
    }

    fn request_current_key(&mut self) {
        self.state = PracticeProgramState::PROMPTING;

        self.say(format!("play {} mayjur", KEYS_IN_CIRCLE_OF_FOURTHS_ORDER[self.current_key]));

        self.state = PracticeProgramState::LISTENING;
    }

    fn advance_current_key(&mut self) {
        if self.current_key + 1 < KEYS_IN_CIRCLE_OF_FOURTHS_ORDER.len() {
            self.current_key += 1;
        } else {
            info!("program finished");
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
                    self.say("You've played a major scale but in the wrong key.".into());
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
