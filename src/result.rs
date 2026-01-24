pub type GenericError = Box<dyn std::error::Error + Send + Sync>;
pub type StdResult<T, E> = std::result::Result<T, E>;
pub type Result<T> = std::result::Result<T, GenericError>;
