#[derive(Debug, PartialEq)]
pub enum DecodeError {
    StringDelimiterNotFound,
    InvalidStringLengthValue {
        bytes: Vec<u8>,
        value: Option<String>,
    },
    StringLengthValueTooBig {
        expected: usize,
        actual: usize,
    },

    EndingDelimiterNotFound,
}

impl std::error::Error for DecodeError {}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
