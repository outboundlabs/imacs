//! Error types for the Espresso library

use thiserror::Error;

/// Errors that can occur during Espresso operations
#[derive(Error, Debug)]
pub enum EspressoError {
    /// Invalid cube format
    #[error("Invalid cube format: {0}")]
    InvalidCube(String),

    /// Invalid PLA format
    #[error("Invalid PLA format at line {line}: {message}")]
    InvalidPla { line: usize, message: String },

    /// Dimension mismatch between cubes or covers
    #[error("Dimension mismatch: expected {expected} inputs, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    /// Output dimension mismatch
    #[error("Output dimension mismatch: expected {expected} outputs, got {got}")]
    OutputMismatch { expected: usize, got: usize },

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),
}
