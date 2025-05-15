// src/db/mod.rs
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::{
    backup::{Backup, StepResult},
    params,
    Connection,
    OpenFlags,
    OptionalExtension,
};
use tracing::{debug, info};

/// Embed every numbered migration file here.
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_initial_schema.sql", include_str!("migrations/0001_initial_schema.sql")),
    ("0002_update_fts_and_triggers.sql", include_str!("migrations/0002_update_fts_and_triggers.sql")),
];

/* ─── connection bootstrap ──────────────────────────────────────────── */

pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let db_path_ref = db_path.as_ref();
    let mut conn = Connection::open(db_path_ref)
        .with_context(|| format!("failed to open DB at {}", db_path_ref.display()))?;

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Apply migrations
    apply_migrations(&mut conn)?;

    Ok(conn)
}

/* ─── migration runner ──────────────────────────────────────────────── */

fn apply_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
             version     INTEGER PRIMARY KEY,
             applied_on  TEXT NOT NULL
         );",
    )?;

    // legacy patch (ignore if already exists)
    let _ = conn.execute("ALTER TABLE schema_version ADD COLUMN applied_on TEXT", []);

    let tx = conn.transaction()?;
    for (fname, sql) in MIGRATIONS {
        let version: i64 = fname
            .split('_')
            .next()
            .and_then(|s| s.parse().ok())
            .expect("migration filenames start with number");

        let already: Option<i64> = tx
            .query_row(
                "SELECT version FROM schema_version WHERE version = ?1",
                [version],
                |r| r.get(0),
            )
            .optional()?;

        if already.is_some() {
            debug!("migration {fname} already applied");
            continue;
        }

        info!("applying migration {fname}");
        // For debugging:
        println!(
            "\nSQL SCRIPT FOR MIGRATION: {}\nBEGIN SQL >>>\n{}\n<<< END SQL\n",
            fname, sql
        );

        tx.execute_batch(sql)
            .with_context(|| format!("could not apply migration {fname}"))?;

        tx.execute(
            "INSERT INTO schema_version (version, applied_on) VALUES (?1, ?2)",
            params![version, Local::now().to_rfc3339()],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/* ─── helpers ───────────────────────────────────────────────────────── */

pub fn ensure_tag_path(conn: &Connection, path: &str) -> Result<i64> {
    let mut parent: Option<i64> = None;
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        conn.execute(
            "INSERT OR IGNORE INTO tags(name, parent_id) VALUES (?1, ?2)",
            params![segment, parent],
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name = ?1 AND (parent_id IS ?2 OR parent_id = ?2)",
            params![segment, parent],
            |row| row.get(0),
        )?;
        parent = Some(id);
    }
    parent.ok_or_else(|| anyhow::anyhow!("empty tag path"))
}

pub fn file_id(conn: &Connection, path: &str) -> Result<i64> {
    conn.query_row("SELECT id FROM files WHERE path = ?1", [path], |r| r.get(0))
        .map_err(|_| anyhow::anyhow!("file not indexed: {}", path))
}

pub fn upsert_attr(conn: &Connection, file_id: i64, key: &str, value: &str) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO attributes(file_id, key, value)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(file_id, key) DO UPDATE SET value = excluded.value
        "#,
        params![file_id, key, value],
    )?;
    Ok(())
}

/* ─── backup / restore ──────────────────────────────────────────────── */

pub fn backup<P: AsRef<Path>>(db_path: P) -> Result<PathBuf> {
    let src = db_path.as_ref();
    let dir = src
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid DB path: {}", src.display()))?
        .join("backups");
    fs::create_dir_all(&dir)?;

    let stamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let dst = dir.join(format!("backup_{stamp}.db"));

    let src_conn = Connection::open_with_flags(src, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut dst_conn = Connection::open(&dst)?;

    let bk = Backup::new(&src_conn, &mut dst_conn)?;
    while let StepResult::More = bk.step(100)? {}
    Ok(dst)
}

pub fn restore<P: AsRef<Path>>(backup_path: P, live_db_path: P) -> Result<()> {
    fs::copy(&backup_path, &live_db_path)?;
    Ok(())
}
