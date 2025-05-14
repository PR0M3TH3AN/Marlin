// src/main.rs
mod cli;
mod config;
mod db;
mod logging;
mod scan;

use anyhow::Result;
use clap::Parser;
use cli::{AttrCmd, Cli, Commands};
use glob::glob;
use rusqlite::params;
use tracing::{error, info};

fn main() -> Result<()> {
    logging::init();

    let args = Cli::parse();
    let cfg = config::Config::load()?;

    // snapshot unless doing an explicit backup / restore
    if !matches!(args.command, Commands::Backup | Commands::Restore { .. }) {
        let _ = db::backup(&cfg.db_path);
    }

    // open database (runs migrations / dynamic column adds)
    let mut conn = db::open(&cfg.db_path)?;

    match args.command {
        Commands::Init => {
            info!("database initialised at {}", cfg.db_path.display());
        }

        Commands::Scan { paths } => {
            if paths.is_empty() {
                anyhow::bail!("At least one directory must be supplied to `scan`");
            }
            for p in paths {
                scan::scan_directory(&mut conn, &p)?;
            }
        }

        Commands::Tag { pattern, tag_path } => apply_tag(&conn, &pattern, &tag_path)?,

        Commands::Attr { action } => match action {
            // borrow the Strings so attr_set gets &str
            AttrCmd::Set { pattern, key, value } => {
                attr_set(&conn, &pattern, &key, &value)?
            }
            AttrCmd::Ls { path } => attr_ls(&conn, &path)?,
        },

        Commands::Search { query, exec } => run_search(&conn, &query, exec)?,

        Commands::Backup => {
            let path = db::backup(&cfg.db_path)?;
            println!("Backup created: {}", path.display());
        }

        Commands::Restore { backup_path } => {
            drop(conn); // close handle
            db::restore(&backup_path, &cfg.db_path)?;
            println!("Restored from {}", backup_path.display());
        }
    }

    Ok(())
}

/* ─── tagging ────────────────────────────────────────────────────────── */
fn apply_tag(conn: &rusqlite::Connection, pattern: &str, tag_path: &str) -> Result<()> {
    let tag_id = db::ensure_tag_path(conn, tag_path)?;
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
                    info!(file = %path_str, tag = tag_path, "tagged");
                } else {
                    error!(file = %path_str, "file not in index – run `marlin scan` first");
                }
            }
            Err(e) => error!(error = %e, "glob error"),
        }
    }
    Ok(())
}

/* ─── attributes ─────────────────────────────────────────────────────── */
fn attr_set(conn: &rusqlite::Connection, pattern: &str, key: &str, value: &str) -> Result<()> {
    for entry in glob(pattern)? {
        match entry {
            Ok(path) => {
                let path_str = path.to_string_lossy();
                let file_id = db::file_id(conn, &path_str)?;
                db::upsert_attr(conn, file_id, key, value)?;
                info!(file = %path_str, key = key, value = value, "attr set");
            }
            Err(e) => error!(error = %e, "glob error"),
        }
    }
    Ok(())
}

fn attr_ls(conn: &rusqlite::Connection, path: &std::path::Path) -> Result<()> {
    let file_id = db::file_id(conn, &path.to_string_lossy())?;
    let mut stmt = conn.prepare("SELECT key, value FROM attributes WHERE file_id = ?1")?;
    let rows = stmt.query_map([file_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows {
        let (k, v) = row?;
        println!("{k} = {v}");
    }
    Ok(())
}

/* ─── search helpers ─────────────────────────────────────────────────── */
fn run_search(conn: &rusqlite::Connection, raw: &str, exec: Option<String>) -> Result<()> {
    let hits = search(conn, raw)?;

    if hits.is_empty() && exec.is_none() {
        eprintln!("No matches for `{}`", raw);
        return Ok(());
    }

    if let Some(cmd_tpl) = exec {
        for path in hits {
            let cmd_final = if cmd_tpl.contains("{}") {
                cmd_tpl.replace("{}", &path)
            } else {
                format!("{cmd_tpl} \"{path}\"")
            };
            let mut parts = cmd_final.splitn(2, ' ');
            let prog = parts.next().unwrap();
            let args = parts.next().unwrap_or("");
            let status = std::process::Command::new(prog)
                .args(shlex::split(args).unwrap_or_default())
                .status()?;
            if !status.success() {
                error!(file = %path, "command failed");
            }
        }
    } else {
        for p in hits {
            println!("{p}");
        }
    }
    Ok(())
}

fn search(conn: &rusqlite::Connection, raw: &str) -> Result<Vec<String>> {
    let q = if raw.split_ascii_whitespace().count() == 1
        && !raw.contains(&['"', '\'', ':', '*', '(', ')', '~', '+', '-'][..])
    {
        format!("{raw}*")
    } else {
        raw.to_string()
    };

    let mut stmt = conn.prepare(
        r#"
        SELECT f.path FROM files_fts
        JOIN files f ON f.rowid = files_fts.rowid
        WHERE files_fts MATCH ?1
        "#,
    )?;
    let rows = stmt.query_map([&q], |row| row.get::<_, String>(0))?;
    Ok(rows.filter_map(Result::ok).collect())
}
