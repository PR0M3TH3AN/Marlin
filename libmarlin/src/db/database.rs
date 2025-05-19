//! Database abstraction for Marlin
//! 
//! This module provides a database abstraction layer that wraps the SQLite connection
//! and provides methods for common database operations.

use rusqlite::Connection;
use std::path::PathBuf;
use anyhow::Result;

/// Options for indexing files
#[derive(Debug, Clone)]
pub struct IndexOptions {
    /// Only update files marked as dirty
    pub dirty_only: bool,
    
    /// Index file contents (not just metadata)
    pub index_contents: bool,
    
    /// Maximum file size to index (in bytes)
    pub max_size: Option<u64>,
}

impl Default for IndexOptions {
    fn default() -> Self {
        Self {
            dirty_only: false,
            index_contents: true,
            max_size: Some(1_000_000), // 1MB default limit
        }
    }
}

/// Database wrapper for Marlin
pub struct Database {
    /// The SQLite connection
    conn: Connection,
}

impl Database {
    /// Create a new database wrapper around an existing connection
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
    
    /// Get a reference to the underlying connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
    
    /// Get a mutable reference to the underlying connection
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
    
    /// Index one or more files
    pub fn index_files(&mut self, paths: &[PathBuf], _options: &IndexOptions) -> Result<usize> {
        // In a real implementation, this would index the files
        // For now, we just return the number of files "indexed"
        Ok(paths.len())
    }
    
    /// Remove files from the index
    pub fn remove_files(&mut self, paths: &[PathBuf]) -> Result<usize> {
        // In a real implementation, this would remove the files
        // For now, we just return the number of files "removed"
        Ok(paths.len())
    }
}
