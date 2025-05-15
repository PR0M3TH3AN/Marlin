// src/main.rs
mod cli;
mod config;
mod db;
mod logging;
mod scan;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, CommandFactory};
use clap_complete::{generate, Shell};
use glob::Pattern;
use rusqlite::params;
use shellexpand;
use shlex;
use std::{env, io, path::PathBuf, process::Command};
use tracing::{debug, error, info};
use walkdir::WalkDir;

use cli::{Cli, Commands, Format};

fn main() -> Result<()> {
    // Parse CLI and bootstrap logging
    let mut args = Cli::parse();
    if args.verbose {
        env::set_var("RUST_LOG", "debug");
    }
    logging::init();

    // If the user asked for completions, generate and exit immediately.
    if let Commands::Completions { shell } = &args.command {
        let mut cmd = Cli::command();
        // Shell is Copy so we can deref it safely
        generate(*shell, &mut cmd, "marlin", &mut io::stdout());
        return Ok(());
    }

    let cfg = config::Config::load()?;

    // Backup before any non-init, non-backup/restore command
    match &args.command {
        Commands::Init | Commands::Backup | Commands::Restore { .. } => {}
        _ => match db::backup(&cfg.db_path) {
            Ok(path) => info!("Pre-command auto-backup created at {}", path.display()),
            Err(e)   => error!("Failed to create pre-command auto-backup: {}", e),
        },
    }

    // Open (and migrate) the DB
    let mut conn = db::open(&cfg.db_path)?;

    // Dispatch all commands
    match args.command {
        Commands::Completions { .. } => {
            // no-op, already handled above
        }
        Commands::Init => {
            info!("Database initialised at {}", cfg.db_path.display());
        }
        Commands::Scan { paths } => {
            let scan_paths = if paths.is_empty() {
                vec![std::env::current_dir()?]
            } else {
                paths
            };
            for p in scan_paths {
                scan::scan_directory(&mut conn, &p)?;
            }
        }
        Commands::Tag { pattern, tag_path } => {
            apply_tag(&conn, &pattern, &tag_path)?;
        }
        Commands::Attr { action } => match action {
            cli::AttrCmd::Set { pattern, key, value } => {
                attr_set(&conn, &pattern, &key, &value)?;
            }
            cli::AttrCmd::Ls { path } => {
                attr_ls(&conn, &path)?;
            }
        },
        Commands::Search { query, exec } => {
            run_search(&conn, &query, exec)?;
        }
        Commands::Backup => {
            let path = db::backup(&cfg.db_path)?;
            println!("Backup created: {}", path.display());
        }
        Commands::Restore { backup_path } => {
            drop(conn);
            db::restore(&backup_path, &cfg.db_path)
                .with_context(|| format!("Failed to restore DB from {}", backup_path.display()))?;
            println!("Restored DB from {}", backup_path.display());
            db::open(&cfg.db_path)
                .with_context(|| format!("Could not open restored DB at {}", cfg.db_path.display()))?;
            info!("Successfully opened restored database.");
        }
        // new domains delegate to their run() functions
        Commands::Link(link_cmd)   => cli::link::run(&link_cmd, &mut conn, args.format)?,
        Commands::Coll(coll_cmd)   => cli::coll::run(&coll_cmd, &mut conn, args.format)?,
        Commands::View(view_cmd)   => cli::view::run(&view_cmd, &mut conn, args.format)?,
        Commands::State(state_cmd) => cli::state::run(&state_cmd, &mut conn, args.format)?,
        Commands::Task(task_cmd)   => cli::task::run(&task_cmd, &mut conn, args.format)?,
        Commands::Remind(rm_cmd)   => cli::remind::run(&rm_cmd, &mut conn, args.format)?,
        Commands::Annotate(an_cmd) => cli::annotate::run(&an_cmd, &mut conn, args.format)?,
        Commands::Version(v_cmd)   => cli::version::run(&v_cmd, &mut conn, args.format)?,
        Commands::Event(e_cmd)     => cli::event::run(&e_cmd, &mut conn, args.format)?,
    }

    Ok(())
}

/// Apply a hierarchical tag to all files matching the glob pattern.
fn apply_tag(conn: &rusqlite::Connection, pattern: &str, tag_path: &str) -> Result<()> {
    let tag_id = db::ensure_tag_path(conn, tag_path)?;
    let expanded = shellexpand::tilde(pattern).into_owned();
    let pat = Pattern::new(&expanded)
        .with_context(|| format!("Invalid glob pattern `{}`", expanded))?;
    let root = determine_scan_root(&expanded);

    let mut stmt_file = conn.prepare("SELECT id FROM files WHERE path = ?1")?;
    let mut stmt_insert =
        conn.prepare("INSERT OR IGNORE INTO file_tags(file_id, tag_id) VALUES (?1, ?2)")?;

    let mut count = 0;
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path_str = entry.path().to_string_lossy();
        debug!("testing path: {}", path_str);
        if !pat.matches(&path_str) {
            debug!("  → no match");
            continue;
        }
        debug!("  → matched");

        match stmt_file.query_row(params![path_str.as_ref()], |r| r.get::<_, i64>(0)) {
            Ok(file_id) => {
                if stmt_insert.execute(params![file_id, tag_id])? > 0 {
                    info!(file = %path_str, tag = tag_path, "tagged");
                    count += 1;
                } else {
                    debug!(file = %path_str, tag = tag_path, "already tagged");
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                error!(file = %path_str, "not indexed – run `marlin scan` first");
            }
            Err(e) => {
                error!(file = %path_str, error = %e, "could not lookup file ID");
            }
        }
    }

    if count > 0 {
        info!("Applied tag '{}' to {} file(s).", tag_path, count);
    } else {
        info!("No new files were tagged with '{}' (no matches or already tagged).", tag_path);
    }
    Ok(())
}

/// Set a key=value attribute on all files matching the glob pattern.
fn attr_set(
    conn: &rusqlite::Connection,
    pattern: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    let expanded = shellexpand::tilde(pattern).into_owned();
    let pat = Pattern::new(&expanded)
        .with_context(|| format!("Invalid glob pattern `{}`", expanded))?;
    let root = determine_scan_root(&expanded);

    let mut stmt_file = conn.prepare("SELECT id FROM files WHERE path = ?1")?;
    let mut count = 0;

    for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()) {
        let path_str = entry.path().to_string_lossy();
        debug!("testing attr path: {}", path_str);
        if !pat.matches(&path_str) {
            debug!("  → no match");
            continue;
        }
        debug!("  → matched");

        match stmt_file.query_row(params![path_str.as_ref()], |r| r.get::<_, i64>(0)) {
            Ok(file_id) => {
                db::upsert_attr(conn, file_id, key, value)?;
                info!(file = %path_str, key = key, value = value, "attr set");
                count += 1;
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                error!(file = %path_str, "not indexed – run `marlin scan` first");
            }
            Err(e) => {
                error!(file = %path_str, error = %e, "could not lookup file ID");
            }
        }
    }

    if count > 0 {
        info!("Attribute '{}: {}' set on {} file(s).", key, value, count);
    } else {
        info!("No attributes set (no matches or not indexed).");
    }
    Ok(())
}

/// List attributes for a given file path.
fn attr_ls(conn: &rusqlite::Connection, path: &std::path::Path) -> Result<()> {
    let file_id = db::file_id(conn, &path.to_string_lossy())?;
    let mut stmt = conn.prepare(
        "SELECT key, value FROM attributes WHERE file_id = ?1 ORDER BY key",
    )?;
    for row in stmt.query_map([file_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (k, v) = row?;
        println!("{k} = {v}");
    }
    Ok(())
}

/// Build and run an FTS5 search query, with optional exec.
fn run_search(conn: &rusqlite::Connection, raw_query: &str, exec: Option<String>) -> Result<()> {
    let mut fts_query_parts = Vec::new();
    let parts = shlex::split(raw_query).unwrap_or_else(|| vec![raw_query.to_string()]);
    for part in parts {
        if ["AND", "OR", "NOT"].contains(&part.as_str()) {
            fts_query_parts.push(part);
        } else if let Some(tag) = part.strip_prefix("tag:") {
            fts_query_parts.push(format!("tags_text:{}", escape_fts_query_term(tag)));
        } else if let Some(attr) = part.strip_prefix("attr:") {
            fts_query_parts.push(format!("attrs_text:{}", escape_fts_query_term(attr)));
        } else {
            fts_query_parts.push(escape_fts_query_term(&part));
        }
    }
    let fts_expr = fts_query_parts.join(" ");
    debug!("Constructed FTS MATCH expression: {}", fts_expr);

    let mut stmt = conn.prepare(
        r#"
        SELECT f.path
          FROM files_fts
          JOIN files f ON f.rowid = files_fts.rowid
         WHERE files_fts MATCH ?1
         ORDER BY rank
        "#,
    )?;
    let hits: Vec<String> = stmt
        .query_map(params![fts_expr], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();

    if let Some(cmd_tpl) = exec {
        let mut ran_without_placeholder = false;
        if hits.is_empty() && !cmd_tpl.contains("{}") {
            if let Some(mut parts) = shlex::split(&cmd_tpl) {
                if !parts.is_empty() {
                    let prog = parts.remove(0);
                    let status = Command::new(&prog).args(&parts).status()?;
                    if !status.success() {
                        error!(command=%cmd_tpl, code=?status.code(), "command failed");
                    }
                }
            }
            ran_without_placeholder = true;
        }
        if !ran_without_placeholder {
            for path in hits {
                let quoted = shlex::try_quote(&path).unwrap_or(path.clone().into());
                let cmd_final = if cmd_tpl.contains("{}") {
                    cmd_tpl.replace("{}", &quoted)
                } else {
                    format!("{} {}", cmd_tpl, &quoted)
                };
                if let Some(mut parts) = shlex::split(&cmd_final) {
                    if parts.is_empty() {
                        continue;
                    }
                    let prog = parts.remove(0);
                    let status = Command::new(&prog).args(&parts).status()?;
                    if !status.success() {
                        error!(file=%path, command=%cmd_final, code=?status.code(), "command failed");
                    }
                }
            }
        }
    } else {
        if hits.is_empty() {
            eprintln!("No matches for query: `{}` (FTS expression: `{}`)", raw_query, fts_expr);
        } else {
            for p in hits {
                println!("{}", p);
            }
        }
    }

    Ok(())
}

/// Quote terms for FTS when needed.
fn escape_fts_query_term(term: &str) -> String {
    if term.contains(|c: char| c.is_whitespace() || "-:()\"".contains(c))
        || ["AND","OR","NOT","NEAR"].contains(&term.to_uppercase().as_str())
    {
        format!("\"{}\"", term.replace('"', "\"\""))
    } else {
        term.to_string()
    }
}

/// Determine a filesystem root to limit recursive walking.
fn determine_scan_root(pattern: &str) -> PathBuf {
    let wildcard_pos = pattern.find(|c| c == '*' || c == '?' || c == '[').unwrap_or(pattern.len());
    let prefix = &pattern[..wildcard_pos];
    let mut root = PathBuf::from(prefix);
    while root.as_os_str().to_string_lossy().contains(|c| ['*','?','['].contains(&c)) {
        if let Some(parent) = root.parent() {
            root = parent.to_path_buf();
        } else {
            root = PathBuf::from(".");
            break;
        }
    }
    root
}
