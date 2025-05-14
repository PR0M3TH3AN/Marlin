// src/db/mod.rs
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::Local;
use rusqlite::{
    backup::{Backup, StepResult},
    params, Connection, OpenFlags,
};

const MIGRATIONS_SQL: &str = include_str!("migrations.sql");

/// Open (or create) the DB, apply migrations, add any missing columns,
/// and rebuild the FTS index if needed.
pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let conn = Connection::open(&db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(MIGRATIONS_SQL)?;

    // example of dynamic column addition: files.hash TEXT
    ensure_column(&conn, "files", "hash", "TEXT")?;

    // ensure FTS picks up tokenizer / prefix changes
    conn.execute("INSERT INTO files_fts(files_fts) VALUES('rebuild')", [])?;
    Ok(conn)
}

/// Add a column if it does not already exist.
fn ensure_column(conn: &Connection, table: &str, col: &str, ddl_type: &str) -> Result<()> {
    // PRAGMA table_info returns rows with (cid, name, type, ...)
    let mut exists = false;
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table});"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in rows.flatten() {
        if name == col {
            exists = true;
            break;
        }
    }

    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {col} {ddl_type};"),
            [],
        )?;
    }
    Ok(())
}

/// Ensure a (possibly hierarchical) tag exists and return the leaf tag id.
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

/// Look up `files.id` by absolute path.
pub fn file_id(conn: &Connection, path: &str) -> Result<i64> {
    conn.query_row("SELECT id FROM files WHERE path = ?1", [path], |r| r.get(0))
        .map_err(|_| anyhow::anyhow!("file not indexed: {}", path))
}

/// Insert or update an attribute.
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

/// Create a **consistent snapshot** of the DB and return the backup path.
pub fn backup<P: AsRef<Path>>(db_path: P) -> Result<PathBuf> {
    let src = db_path.as_ref();
    let dir = src
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid DB path"))?
        .join("backups");
    fs::create_dir_all(&dir)?;

    let stamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let dst = dir.join(format!("backup_{stamp}.db"));

    // open connections: src read-only, dst writable
    let src_conn = Connection::open_with_flags(src, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut dst_conn = Connection::open(&dst)?;

    // run online backup
    let mut bk = Backup::new(&src_conn, &mut dst_conn)?;
    while let StepResult::More = bk.step(100)? {}
    // Backup finalised when `bk` is dropped.

    Ok(dst)
}

/// Replace the live DB file with a snapshot (caller must have closed handles).
pub fn restore<P: AsRef<Path>>(backup_path: P, live_db_path: P) -> Result<()> {
    fs::copy(&backup_path, &live_db_path)?;
    Ok(())
}
