#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticeProgramState {
    INITIALIZING,
    LISTENING,
    PROMPTING,
    FINISHED
}

pub trait PracticeProgram {
    fn get_state(&self) -> PracticeProgramState;

    fn run(&mut self);
}

pub struct CircleOfFourthsPracticeProgram {
    current_key: String,
    state: PracticeProgramState
}

impl PracticeProgram for CircleOfFourthsPracticeProgram {
    fn get_state(&self) -> PracticeProgramState {
        return self.state
    }

    fn run(&mut self) {
        self.state = PracticeProgramState::LISTENING
    }
}

impl CircleOfFourthsPracticeProgram {
    pub fn new() -> CircleOfFourthsPracticeProgram {
        CircleOfFourthsPracticeProgram { current_key: String::from("C"), state: PracticeProgramState::INITIALIZING }
    }
}