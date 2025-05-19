//! Error types for Marlin
//!
//! This module defines custom error types used throughout the application.

use std::io;
use std::fmt;

/// Result type for Marlin - convenience wrapper around Result<T, Error>
pub type Result<T> = std::result::Result<T, Error>;

/// Custom error types for Marlin
#[derive(Debug)]
pub enum Error {
    /// An IO error
    Io(io::Error),
    
    /// A database error
    Database(String),
    
    /// An error from the notify library
    Watch(String),
    
    /// Invalid state for the requested operation
    InvalidState(String),
    
    /// Path not found
    NotFound(String),
    
    /// Invalid configuration
    Config(String),
    
    /// Other errors
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::Watch(msg) => write!(f, "Watch error: {}", msg),
            Self::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Self::NotFound(path) => write!(f, "Not found: {}", path),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Self::Watch(err.to_string())
    }
}
