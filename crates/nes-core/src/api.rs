use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Pause,
    Resume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreQuery {
    EmulatorState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmulatorState {
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryResult {
    EmulatorState(EmulatorState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NesCore {
    paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreError {
    UnsupportedCommand,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCommand => f.write_str("unsupported command"),
        }
    }
}

impl std::error::Error for CoreError {}

impl NesCore {
    #[must_use]
    pub fn new() -> Self {
        Self { paused: false }
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn execute(&mut self, command: Command) -> Result<(), CoreError> {
        match command {
            Command::Pause => {
                self.paused = true;
                Ok(())
            }
            Command::Resume => {
                self.paused = false;
                Ok(())
            }
        }
    }

    #[must_use]
    pub fn query(&self, query: CoreQuery) -> QueryResult {
        match query {
            CoreQuery::EmulatorState => QueryResult::EmulatorState(EmulatorState {
                paused: self.paused,
            }),
        }
    }
}

impl Default for NesCore {
    fn default() -> Self {
        Self::new()
    }
}
