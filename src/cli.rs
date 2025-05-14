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

    /// Scan one or more directories and populate the file index
    ///
    /// Example:
    ///     marlin scan ~/Pictures ~/Documents ~/Downloads
    Scan {
        /// One or more directories to walk
        paths: Vec<PathBuf>,
    },

    /// Tag files matching a glob pattern
    ///
    /// Example:
    ///     marlin tag "~/Pictures/**/*.jpg" vacation
    Tag {
        /// Glob pattern (quote to avoid shell expansion)
        pattern: String,
        /// Tag name
        tag: String,
    },
}
