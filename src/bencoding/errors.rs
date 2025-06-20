#[derive(Debug, PartialEq)]
pub enum DecodeError {
    StringDelimiterNotFound,
    InvalidStringLengthValue(String),
    StringLengthValueTooBig { expected: usize, actual: usize },

    InvalidIntValue(String),

    EndingDelimiterNotFound,
}

impl std::error::Error for DecodeError {}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
