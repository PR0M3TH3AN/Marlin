// src/cli.rs
use std::path::PathBuf;
use clap::{Parser, Subcommand};

/// Marlin – metadata-driven file explorer (CLI utilities)
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    /// Enable debug logging and extra output
    #[arg(long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialise the database (idempotent)
    Init,

    /// Scan one or more directories and populate the file index
    Scan {
        paths: Vec<PathBuf>,
    },

    /// Tag files matching a glob pattern (hierarchical tags use `/`)
    Tag {
        pattern: String,
        tag_path: String,
    },

    /// Manage custom attributes
    Attr {
        #[command(subcommand)]
        action: AttrCmd,
    },

    /// Full-text search; `--exec CMD` runs CMD on each hit (`{}` placeholder)
    Search {
        query: String,
        #[arg(long)]
        exec: Option<String>,
    },

    /// Create a timestamped backup of the database
    Backup,

    /// Restore from a backup file (over-writes current DB)
    Restore {
        backup_path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum AttrCmd {
    Set { pattern: String, key: String, value: String },
    Ls  { path: PathBuf },
}
