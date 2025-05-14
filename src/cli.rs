use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Marlin – metadata-driven file explorer (CLI utilities)
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialise the database (idempotent)
    Init,
    /// Scan a directory and populate the file index
    Scan {
        /// Directory to walk
        path: PathBuf,
    },
    /// Tag files matching a glob pattern
    Tag {
        /// Glob pattern (quote to avoid shell expansion)
        pattern: String,
        /// Tag name
        tag: String,
    },
}
