#[derive(Debug, PartialEq)]
pub enum Error {
    StringDelimiterNotFound,
    InvalidStringLengthValue(String),
    StringLengthValueTooBig { expected: usize, actual: usize },

    InvalidIntValue(String),

    EndingDelimiterNotFound,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
