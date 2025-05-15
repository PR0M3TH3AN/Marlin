// src/cli.rs
pub mod link;
pub mod coll;
pub mod view;
pub mod state;
pub mod task;
pub mod remind;
pub mod annotate;
pub mod version;
pub mod event;

use clap::{Parser, Subcommand, ArgEnum, Args, CommandFactory};
use clap_complete::Shell;

/// Output format for commands.
#[derive(ArgEnum, Clone, Copy, Debug)]
pub enum Format {
    Text,
    Json,
}

/// Marlin – metadata-driven file explorer (CLI utilities)
#[derive(Parser, Debug)]
#[command(author, version, about, propagate_version = true)]
pub struct Cli {
    /// Enable debug logging and extra output
    #[arg(long)]
    pub verbose: bool,

    /// Output format (text or JSON)
    #[arg(long, default_value = "text", value_enum, global = true)]
    pub format: Format,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialise the database (idempotent)
    Init,

    /// Scan one or more directories and populate the file index
    Scan {
        /// Directories to scan (defaults to cwd)
        paths: Vec<std::path::PathBuf>,
    },

    /// Tag files matching a glob pattern (hierarchical tags use `/`)
    Tag {
        /// Glob or path pattern
        pattern: String,
        /// Hierarchical tag name (`foo/bar`)
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

    /// Restore from a backup file (overwrites current DB)
    Restore {
        backup_path: std::path::PathBuf,
    },

    /// Generate shell completions (hidden)
    #[command(hide = true)]
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },

    /// File-to-file links
    #[command(subcommand)]
    Link   { cmd: link::LinkCmd },

    /// Collections (groups) of files
    #[command(subcommand)]
    Coll   { cmd: coll::CollCmd },

    /// Smart views (saved queries)
    #[command(subcommand)]
    View   { cmd: view::ViewCmd },

    /// Workflow states on files
    #[command(subcommand)]
    State  { cmd: state::StateCmd },

    /// TODO/tasks management
    #[command(subcommand)]
    Task   { cmd: task::TaskCmd },

    /// Reminders on files
    #[command(subcommand)]
    Remind { cmd: remind::RemindCmd },

    /// File annotations and highlights
    #[command(subcommand)]
    Annotate { cmd: annotate::AnnotateCmd },

    /// Version diffs
    #[command(subcommand)]
    Version { cmd: version::VersionCmd },

    /// Calendar events & timelines
    #[command(subcommand)]
    Event { cmd: event::EventCmd },
}

#[derive(Subcommand, Debug)]
pub enum AttrCmd {
    Set { pattern: String, key: String, value: String },
    Ls  { path: std::path::PathBuf },
}
