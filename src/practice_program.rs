use std::{
    sync::mpsc::SyncSender,
    sync::{mpsc::Receiver, Arc},
};

use log::info;

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
        let harmonic_minor_scale_deltas = [2, 1, 2, 2, 1, 3, 1];

        if crate::music::scale_matches_increments(&kmsg_log, major_scale_deltas) {
            log::info!(
                "user played major scale starting at {}",
                kmsg_log[0].readable_note()
            );
            self.ctrl_sender.send(ControlMessage::NewRun).unwrap();
        }

        if crate::music::scale_matches_increments(&kmsg_log, harmonic_minor_scale_deltas) {
            log::info!(
                "user played harmonic minor scale starting at {}",
                kmsg_log[0].readable_note()
            );
            self.ctrl_sender.send(ControlMessage::NewRun).unwrap();
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
    current_key: String,
}

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
            current_key: String::from("C"),
        }
    }

    fn request_current_key(&mut self) {
        self.state = PracticeProgramState::PROMPTING;

        std::process::Command::new("say")
            .arg("--voice=Moira")
            .arg(format!("play {} mayjur", self.current_key))
            .spawn()
            .unwrap();

        self.state = PracticeProgramState::LISTENING;
    }

    fn advance_current_key(&mut self) {
        if self.current_key == "C" {
            self.current_key = String::from("F");
        }
    }

    fn on_keypress(&mut self, _latest: KeyMessage) {
        let kmsg_log = self.key_db.flat_message_log();
        let major_scale_deltas = [2, 2, 1, 2, 2, 2, 1];

        if crate::music::scale_matches_increments(&kmsg_log, major_scale_deltas) {
            log::info!(
                "user played major scale starting at {}",
                kmsg_log[0].readable_note()
            );

            self.ctrl_sender.send(ControlMessage::NewRun).unwrap();

            self.advance_current_key();
            self.request_current_key();
        }
    }
}

impl PracticeProgram for CircleOfFourthsPracticeProgram {
    fn get_state(&self) -> PracticeProgramState {
        return self.state;
    }

    fn run(mut self) {
        self.request_current_key();
        self.state = PracticeProgramState::LISTENING;
        std::thread::spawn(move || loop {
            let msg = self.key_receiver.recv().unwrap();
            self.on_keypress(msg);
        });
    }
}
