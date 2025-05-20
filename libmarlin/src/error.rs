// libmarlin/src/error.rs

use std::io;
use std::fmt;
// Ensure these are present if Error enum variants use them directly
// use rusqlite;
// use notify;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Database(rusqlite::Error), 
    Watch(notify::Error),    
    InvalidState(String),
    NotFound(String),
    Config(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Database(err) => write!(f, "Database error: {}", err),
            Self::Watch(err) => write!(f, "Watch error: {}", err),
            Self::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Self::NotFound(path) => write!(f, "Not found: {}", path),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Database(err) => Some(err),
            Self::Watch(err) => Some(err),
            Self::InvalidState(_) | Self::NotFound(_) | Self::Config(_) | Self::Other(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err)
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Self::Watch(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError; 

    #[test]
    fn test_error_display_and_from() {
        // Test Io variant
        let io_err_inner_for_source_check = io::Error::new(io::ErrorKind::NotFound, "test io error");
        let io_err_marlin = Error::from(io::Error::new(io::ErrorKind::NotFound, "test io error"));
        assert_eq!(io_err_marlin.to_string(), "IO error: test io error");
        let source = io_err_marlin.source();
        assert!(source.is_some(), "Io error should have a source");
        if let Some(s) = source {
            // Compare details of the source if necessary, or just its string representation
            assert_eq!(s.to_string(), io_err_inner_for_source_check.to_string());
        }

        // Test Database variant
        let rusqlite_err_inner_for_source_check = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR), 
            Some("test db error".to_string()),
        );
        // We need to create the error again for the From conversion if we want to compare the source
        let db_err_marlin = Error::from(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR), 
            Some("test db error".to_string()),
        ));
        assert!(db_err_marlin.to_string().contains("Database error: test db error"));
        let source = db_err_marlin.source();
        assert!(source.is_some(), "Database error should have a source");
        if let Some(s) = source {
            assert_eq!(s.to_string(), rusqlite_err_inner_for_source_check.to_string());
        }


        // Test Watch variant
        let notify_raw_err_inner_for_source_check = notify::Error::new(notify::ErrorKind::Generic("test watch error".to_string()));
        let watch_err_marlin = Error::from(notify::Error::new(notify::ErrorKind::Generic("test watch error".to_string())));
        assert!(watch_err_marlin.to_string().contains("Watch error: test watch error"));
        let source = watch_err_marlin.source();
        assert!(source.is_some(), "Watch error should have a source");
        if let Some(s) = source {
            assert_eq!(s.to_string(), notify_raw_err_inner_for_source_check.to_string());
        }


        let invalid_state_err = Error::InvalidState("bad state".to_string());
        assert_eq!(invalid_state_err.to_string(), "Invalid state: bad state");
        assert!(invalid_state_err.source().is_none());

        let not_found_err = Error::NotFound("missing_file.txt".to_string());
        assert_eq!(not_found_err.to_string(), "Not found: missing_file.txt");
        assert!(not_found_err.source().is_none());

        let config_err = Error::Config("bad config".to_string());
        assert_eq!(config_err.to_string(), "Configuration error: bad config");
        assert!(config_err.source().is_none());

        let other_err = Error::Other("some other issue".to_string());
        assert_eq!(other_err.to_string(), "Error: some other issue");
        assert!(other_err.source().is_none());
    }

    #[test]
    fn test_rusqlite_error_without_message() {
        let sqlite_busy_error = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            None,
        );
        let db_err_no_msg = Error::from(sqlite_busy_error);
        
        let expected_rusqlite_msg = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            None,
        ).to_string(); 
        
        let expected_marlin_msg = format!("Database error: {}", expected_rusqlite_msg);
        
        // Verify the string matches the expected format
        assert_eq!(db_err_no_msg.to_string(), expected_marlin_msg);
        
        // Check the error code directly instead of the string
        if let Error::Database(rusqlite::Error::SqliteFailure(err, _)) = &db_err_no_msg {
            assert_eq!(err.code, rusqlite::ffi::ErrorCode::DatabaseBusy);
        } else {
            panic!("Expected Error::Database variant");
        }
        
        // Verify the source exists
        assert!(db_err_no_msg.source().is_some());
    }
}