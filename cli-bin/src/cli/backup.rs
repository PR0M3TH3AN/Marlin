// src/cli/backup.rs
use crate::cli::Format;
use anyhow::{Context, Result};
use clap::Args;
use libmarlin::backup::BackupManager;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Options for the `backup` command
#[derive(Args, Debug)]
pub struct BackupOpts {
    /// Directory to store backups (defaults next to DB)
    #[arg(long)]
    pub dir: Option<PathBuf>,

    /// Keep only N newest backups
    #[arg(long)]
    pub prune: Option<usize>,

    /// Verify a backup file
    #[arg(long)]
    pub verify: bool,

    /// Backup file to verify (used with --verify)
    #[arg(long)]
    pub file: Option<PathBuf>,
}

pub fn run(opts: &BackupOpts, db_path: &Path, _conn: &mut Connection, _fmt: Format) -> Result<()> {
    let backups_dir = opts
        .dir
        .clone()
        .unwrap_or_else(|| db_path.parent().unwrap().join("backups"));
    let manager = BackupManager::new(db_path, &backups_dir)?;

    if opts.verify {
        let file = opts
            .file
            .as_ref()
            .context("--file required with --verify")?;
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .context("invalid backup file name")?;
        let ok = manager.verify_backup(name)?;
        if ok {
            println!("Backup OK: {}", name);
        } else {
            println!("Backup corrupted: {}", name);
        }
        return Ok(());
    }

    if let Some(n) = opts.prune {
        let result = manager.prune(n)?;
        println!(
            "Pruned {} old backups, kept {}",
            result.removed.len(),
            result.kept.len()
        );
        return Ok(());
    }

    let info = manager.create_backup()?;
    println!("Created backup {}", info.id);
    Ok(())
}
