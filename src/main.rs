mod cli;
mod config;
mod db;
mod logging;
mod scan;

use anyhow::Result;
use clap::Parser;             // 👈 bring in the trait that adds `.parse()`
use cli::{Cli, Commands};
use glob::glob;
use rusqlite::params;
use tracing::{error, info};

fn main() -> Result<()> {
    logging::init();

    let args = Cli::parse();  // now compiles
    let cfg = config::Config::load()?;
    let mut conn = db::open(&cfg.db_path)?;   // mutable

    match args.command {
        Commands::Init => {
            info!("database initialised at {}", cfg.db_path.display());
        }
        Commands::Scan { path } => {
            scan::scan_directory(&mut conn, &path)?;     // pass &mut
        }
        Commands::Tag { pattern, tag } => {
            apply_tag(&conn, &pattern, &tag)?;
        }
    }

    Ok(())
}

/// Apply `tag` to every file that matches `pattern`.
fn apply_tag(conn: &rusqlite::Connection, pattern: &str, tag: &str) -> Result<()> {
    let tag_id = db::ensure_tag(conn, tag)?;
    let mut stmt_file = conn.prepare("SELECT id FROM files WHERE path = ?1")?;
    let mut stmt_insert =
        conn.prepare("INSERT OR IGNORE INTO file_tags(file_id, tag_id) VALUES (?1, ?2)")?;

    for entry in glob(pattern)? {
        match entry {
            Ok(path) => {
                let path_str = path.to_string_lossy();
                if let Ok(file_id) =
                    stmt_file.query_row(params![path_str], |row| row.get::<_, i64>(0))
                {
                    stmt_insert.execute(params![file_id, tag_id])?;
                    info!(file = %path_str, tag = tag, "tagged");
                } else {
                    error!(file = %path_str, "file not in index – run `marlin scan` first");
                }
            }
            Err(e) => error!(error = %e, "glob error"),
        }
    }
    Ok(())
}
