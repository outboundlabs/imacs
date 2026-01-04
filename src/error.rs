//! Error types for IMACS

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// IMACS errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("Spec parse error: {0}")]
    SpecParse(String),

    #[error("Code parse error: {0}")]
    CodeParse(String),

    #[error("CEL parse error: {0}")]
    CelParse(String),

    #[error("CEL evaluation error: {0}")]
    CelEval(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Render error: {0}")]
    Render(String),

    #[error("Pattern error: {0}")]
    Pattern(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
