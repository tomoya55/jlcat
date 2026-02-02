use thiserror::Error;

#[derive(Error, Debug)]
pub enum JlcatError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error at line {line}: {message}")]
    JsonParse { line: usize, message: String },

    #[error("Invalid column path: {0}")]
    InvalidColumnPath(String),

    #[error("Invalid filter expression: {0}")]
    InvalidFilter(String),

    #[error("Invalid sort key: {0}")]
    InvalidSortKey(String),
}

pub type Result<T> = std::result::Result<T, JlcatError>;
