use std::fmt;

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    InvalidConfig(String),
    InvalidDeltaTime,
    Runtime(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(f, "invalid engine config: {message}"),
            Self::InvalidDeltaTime => write!(f, "delta time must be finite and non-negative"),
            Self::Runtime(message) => write!(f, "runtime error: {message}"),
        }
    }
}

impl std::error::Error for EngineError {}
