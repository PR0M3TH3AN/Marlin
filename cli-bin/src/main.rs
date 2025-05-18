//! Marlin CLI entry-point (post crate-split)
//!
//! All heavy lifting now lives in the `libmarlin` crate; this file
//! handles argument parsing, logging, orchestration and the few
//! helpers that remain CLI-specific.

#![deny(warnings)]

mod cli; // sub-command definitions and argument structs

/* ── shared modules re-exported from libmarlin ─────────────────── */
use libmarlin::{
    config,
    db,
    logging,
    scan,
    utils::determine_scan_root,
};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use glob::Pattern;
use shellexpand;
use shlex;
use std::{
    env,
    fs,
    io,
    path::Path,
    process::Command,
};
use tracing::{debug, error, info};
use walkdir::WalkDir;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    /* ── CLI parsing & logging ────────────────────────────────── */

    let args = Cli::parse();
    if args.verbose {
        env::set_var("RUST_LOG", "debug");
    }
    logging::init();

    /* ── shell-completion shortcut ────────────────────────────── */

    if let Commands::Completions { shell } = &args.command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "marlin", &mut io::stdout());
        return Ok(());
    }

    /* ── config & automatic backup ───────────────────────────── */

    let cfg = config::Config::load()?; // resolves DB path

    match &args.command {
        Commands::Init | Commands::Backup | Commands::Restore { .. } => {}
        _ => match db::backup(&cfg.db_path) {
            Ok(p)  => info!("Pre-command auto-backup created at {}", p.display()),
            Err(e) => error!("Failed to create pre-command auto-backup: {e}"),
        },
    }

    /* ── open DB (runs migrations) ───────────────────────────── */

    let mut conn = db::open(&cfg.db_path)?;

    /* ── command dispatch ────────────────────────────────────── */

    match args.command {
        Commands::Completions { .. } => {} // handled above

        /* ---- init ------------------------------------------------ */
        Commands::Init => {
            info!("Database initialised at {}", cfg.db_path.display());
            let cwd   = env::current_dir().context("getting current directory")?;
            let count = scan::scan_directory(&mut conn, &cwd)
                .context("initial scan failed")?;
            info!("Initial scan complete – indexed/updated {count} files");
        }

        /* ---- scan ------------------------------------------------ */
        Commands::Scan { paths } => {
            let scan_paths = if paths.is_empty() {
                vec![env::current_dir()?]
            } else { paths };

            for p in scan_paths {
                scan::scan_directory(&mut conn, &p)?;
            }
        }

        /* ---- tag / attribute / search --------------------------- */
        Commands::Tag { pattern, tag_path } =>
            apply_tag(&conn, &pattern, &tag_path)?,

        Commands::Attr { action } => match action {
            cli::AttrCmd::Set { pattern, key, value } =>
                attr_set(&conn, &pattern, &key, &value)?,
            cli::AttrCmd::Ls { path } =>
                attr_ls(&conn, &path)?,
        },

        Commands::Search { query, exec } =>
            run_search(&conn, &query, exec)?,

        /* ---- maintenance ---------------------------------------- */
        Commands::Backup => {
            let p = db::backup(&cfg.db_path)?;
            println!("Backup created: {}", p.display());
        }

        Commands::Restore { backup_path } => {
            drop(conn); // close handle before overwrite
            db::restore(&backup_path, &cfg.db_path).with_context(|| {
                format!("Failed to restore DB from {}", backup_path.display())
            })?;
            println!("Restored DB from {}", backup_path.display());
            db::open(&cfg.db_path).with_context(|| {
                format!("Could not open restored DB at {}", cfg.db_path.display())
            })?;
            info!("Successfully opened restored database.");
        }

        /* ---- passthrough sub-modules (some still stubs) ---------- */
        Commands::Link(link_cmd)   => cli::link::run(&link_cmd,   &mut conn, args.format)?,
        Commands::Coll(coll_cmd)   => cli::coll::run(&coll_cmd,   &mut conn, args.format)?,
        Commands::View(view_cmd)   => cli::view::run(&view_cmd,   &mut conn, args.format)?,
        Commands::State(state_cmd) => cli::state::run(&state_cmd, &mut conn, args.format)?,
        Commands::Task(task_cmd)   => cli::task::run(&task_cmd,   &mut conn, args.format)?,
        Commands::Remind(rm_cmd)   => cli::remind::run(&rm_cmd,   &mut conn, args.format)?,
        Commands::Annotate(a_cmd)  => cli::annotate::run(&a_cmd,  &mut conn, args.format)?,
        Commands::Version(v_cmd)   => cli::version::run(&v_cmd,   &mut conn, args.format)?,
        Commands::Event(e_cmd)     => cli::event::run(&e_cmd,     &mut conn, args.format)?,
    }

    Ok(())
}

/* ─────────────────── helpers & sub-routines ─────────────────── */

/* ---------- TAGS ---------- */

fn apply_tag(conn: &rusqlite::Connection, pattern: &str, tag_path: &str) -> Result<()> {
    // ensure_tag_path returns ID of deepest node
    let leaf_tag_id = db::ensure_tag_path(conn, tag_path)?;

    // collect leaf + ancestors
    let mut tag_ids = Vec::new();
    let mut current = Some(leaf_tag_id);
    while let Some(id) = current {
        tag_ids.push(id);
        current = conn.query_row(
            "SELECT parent_id FROM tags WHERE id=?1",
            [id],
            |r| r.get::<_, Option<i64>>(0),
        )?;
    }

    let expanded = shellexpand::tilde(pattern).into_owned();
    let pat  = Pattern::new(&expanded)
        .with_context(|| format!("Invalid glob pattern `{expanded}`"))?;
    let root = determine_scan_root(&expanded);

    let mut stmt_file   = conn.prepare("SELECT id FROM files WHERE path=?1")?;
    let mut stmt_insert = conn.prepare(
        "INSERT OR IGNORE INTO file_tags(file_id, tag_id) VALUES (?1, ?2)",
    )?;

    let mut count = 0usize;
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let p = entry.path().to_string_lossy();
        if !pat.matches(&p) { continue; }

        match stmt_file.query_row([p.as_ref()], |r| r.get::<_, i64>(0)) {
            Ok(fid) => {
                let mut newly = false;
                for &tid in &tag_ids {
                    if stmt_insert.execute([fid, tid])? > 0 {
                        newly = true;
                    }
                }
                if newly {
                    info!(file=%p, tag=tag_path, "tagged");
                    count += 1;
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) =>
                error!(file=%p, "not indexed – run `marlin scan` first"),
            Err(e) =>
                error!(file=%p, error=%e, "could not lookup file ID"),
        }
    }

    info!("Applied tag '{}' to {} file(s).", tag_path, count);
    Ok(())
}

/* ---------- ATTRIBUTES ---------- */

fn attr_set(conn: &rusqlite::Connection, pattern: &str, key: &str, value: &str) -> Result<()> {
    let expanded = shellexpand::tilde(pattern).into_owned();
    let pat  = Pattern::new(&expanded)
        .with_context(|| format!("Invalid glob pattern `{expanded}`"))?;
    let root = determine_scan_root(&expanded);

    let mut stmt_file = conn.prepare("SELECT id FROM files WHERE path=?1")?;
    let mut count = 0usize;

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let p = entry.path().to_string_lossy();
        if !pat.matches(&p) { continue; }

        match stmt_file.query_row([p.as_ref()], |r| r.get::<_, i64>(0)) {
            Ok(fid) => {
                db::upsert_attr(conn, fid, key, value)?;
                info!(file=%p, key, value, "attr set");
                count += 1;
            }
            Err(rusqlite::Error::QueryReturnedNoRows) =>
                error!(file=%p, "not indexed – run `marlin scan` first"),
            Err(e) =>
                error!(file=%p, error=%e, "could not lookup file ID"),
        }
    }

    info!("Attribute '{}={}' set on {} file(s).", key, value, count);
    Ok(())
}

fn attr_ls(conn: &rusqlite::Connection, path: &Path) -> Result<()> {
    let fid = db::file_id(conn, &path.to_string_lossy())?;
    let mut stmt = conn.prepare(
        "SELECT key, value FROM attributes WHERE file_id=?1 ORDER BY key"
    )?;
    for row in stmt
        .query_map([fid], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
    {
        let (k, v) = row?;
        println!("{k} = {v}");
    }
    Ok(())
}

/* ---------- SEARCH ---------- */

fn run_search(conn: &rusqlite::Connection, raw_query: &str, exec: Option<String>) -> Result<()> {
    /* ── build FTS expression -------------------------------- */
    let mut parts = Vec::new();
    let toks = shlex::split(raw_query).unwrap_or_else(|| vec![raw_query.to_string()]);
    for tok in toks {
        if ["AND", "OR", "NOT"].contains(&tok.as_str()) {
            parts.push(tok);
        } else if let Some(tag) = tok.strip_prefix("tag:") {
            for (i, seg) in tag.split('/').filter(|s| !s.is_empty()).enumerate() {
                if i > 0 { parts.push("AND".into()); }
                parts.push(format!("tags_text:{}", escape_fts(seg)));
            }
        } else if let Some(attr) = tok.strip_prefix("attr:") {
            let mut kv = attr.splitn(2, '=');
            let key = kv.next().unwrap();
            if let Some(val) = kv.next() {
                parts.push(format!("attrs_text:{}", escape_fts(key)));
                parts.push("AND".into());
                parts.push(format!("attrs_text:{}", escape_fts(val)));
            } else {
                parts.push(format!("attrs_text:{}", escape_fts(key)));
            }
        } else {
            parts.push(escape_fts(&tok));
        }
    }
    let fts_expr = parts.join(" ");
    debug!("FTS MATCH expression: {fts_expr}");

    /* ── primary FTS query ---------------------------------- */
    let mut stmt = conn.prepare(
        r#"
        SELECT f.path
          FROM files_fts
          JOIN files f ON f.rowid = files_fts.rowid
         WHERE files_fts MATCH ?1
         ORDER BY rank
        "#,
    )?;
    let mut hits: Vec<String> = stmt
        .query_map([&fts_expr], |r| r.get::<_, String>(0))?
        .filter_map(Result::ok)
        .collect();

    /* ── graceful fallback (substring scan) ----------------- */
    if hits.is_empty() && !raw_query.contains(':') {
        hits = naive_substring_search(conn, raw_query)?;
    }

    /* ── output / exec -------------------------------------- */
    if let Some(cmd_tpl) = exec {
        run_exec(&hits, &cmd_tpl)?;
    } else {
        if hits.is_empty() {
            eprintln!(
                "No matches for query: `{raw_query}` (FTS expr: `{fts_expr}`)"
            );
        } else {
            for p in hits { println!("{p}"); }
        }
    }
    Ok(())
}

/// Fallback: case-insensitive substring scan over paths *and* small file bodies.
fn naive_substring_search(conn: &rusqlite::Connection, term: &str) -> Result<Vec<String>> {
    let needle = term.to_lowercase();
    let mut stmt = conn.prepare("SELECT path FROM files")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;

    let mut out = Vec::new();
    for p in rows {
        let p = p?;
        if p.to_lowercase().contains(&needle) {
            out.push(p.clone());
            continue;
        }
        // Only scan files ≤ 64 kB
        if let Ok(meta) = fs::metadata(&p) {
            if meta.len() > 65_536 { continue; }
        }
        if let Ok(body) = fs::read_to_string(&p) {
            if body.to_lowercase().contains(&needle) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

/// Run external command template on every hit (`{}` placeholder supported).
fn run_exec(paths: &[String], cmd_tpl: &str) -> Result<()> {
    let mut ran_without_placeholder = false;

    // optimisation: if no hits and no placeholder, run once
    if paths.is_empty() && !cmd_tpl.contains("{}") {
        if let Some(mut parts) = shlex::split(cmd_tpl) {
            if !parts.is_empty() {
                let prog = parts.remove(0);
                let status = Command::new(&prog).args(parts).status()?;
                if !status.success() {
                    error!(command=%cmd_tpl, code=?status.code(), "command failed");
                }
            }
        }
        ran_without_placeholder = true;
    }

    if !ran_without_placeholder {
        for p in paths {
            let quoted = shlex::try_quote(p).unwrap_or_else(|_| p.into());
            let final_cmd = if cmd_tpl.contains("{}") {
                cmd_tpl.replace("{}", &quoted)
            } else {
                format!("{cmd_tpl} {quoted}")
            };
            if let Some(mut parts) = shlex::split(&final_cmd) {
                if parts.is_empty() { continue; }
                let prog = parts.remove(0);
                let status = Command::new(&prog).args(parts).status()?;
                if !status.success() {
                    error!(file=%p, command=%final_cmd, code=?status.code(), "command failed");
                }
            }
        }
    }
    Ok(())
}

/* ---------- misc helpers ---------- */

fn escape_fts(term: &str) -> String {
    if term.contains(|c: char| c.is_whitespace() || "-:()\"".contains(c))
        || ["AND", "OR", "NOT", "NEAR"].contains(&term.to_uppercase().as_str())
    {
        format!("\"{}\"", term.replace('"', "\"\""))
    } else { term.to_string() }
}
