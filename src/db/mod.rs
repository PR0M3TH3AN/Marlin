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
    ("0003_create_links_collections_views.sql", include_str!("migrations/0003_create_links_collections_views.sql")),
];

/* ─── connection bootstrap ──────────────────────────────────────────── */

pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let db_path_ref = db_path.as_ref();
    let mut conn = Connection::open(db_path_ref)
        .with_context(|| format!("failed to open DB at {}", db_path_ref.display()))?;

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Apply migrations (drops & recreates all FTS triggers)
    apply_migrations(&mut conn)?;

    Ok(conn)
}

/* ─── migration runner ──────────────────────────────────────────────── */

fn apply_migrations(conn: &mut Connection) -> Result<()> {
    // Ensure schema_version table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
             version     INTEGER PRIMARY KEY,
             applied_on  TEXT NOT NULL
         );",
    )?;

    // Legacy patch (ignore if exists)
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
            debug!("migration {} already applied", fname);
            continue;
        }

        info!("applying migration {}", fname);
        println!(
            "\nSQL SCRIPT FOR MIGRATION: {}\nBEGIN SQL >>>\n{}\n<<< END SQL\n",
            fname, sql
        );

        tx.execute_batch(sql)
            .with_context(|| format!("could not apply migration {}", fname))?;

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

/// Add a typed link from one file to another.
pub fn add_link(conn: &Connection, src_file_id: i64, dst_file_id: i64, link_type: Option<&str>) -> Result<()> {
    conn.execute(
        "INSERT INTO links(src_file_id, dst_file_id, type)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(src_file_id, dst_file_id, type) DO NOTHING",
        params![src_file_id, dst_file_id, link_type],
    )?;
    Ok(())
}

/// Remove a typed link between two files.
pub fn remove_link(conn: &Connection, src_file_id: i64, dst_file_id: i64, link_type: Option<&str>) -> Result<()> {
    conn.execute(
        "DELETE FROM links
         WHERE src_file_id = ?1
           AND dst_file_id = ?2
           AND (type IS ?3 OR type = ?3)",
        params![src_file_id, dst_file_id, link_type],
    )?;
    Ok(())
}

/// List all links for files matching a glob-style pattern.
/// `direction` may be `"in"` (incoming), `"out"` (outgoing), or `None` (outgoing).
pub fn list_links(
    conn: &Connection,
    pattern: &str,
    direction: Option<&str>,
    link_type: Option<&str>,
) -> Result<Vec<(String, String, Option<String>)>> {
    // Convert glob '*' → SQL LIKE '%'
    let like_pattern = pattern.replace('*', "%");

    // Find matching files
    let mut stmt = conn.prepare("SELECT id, path FROM files WHERE path LIKE ?1")?;
    let mut rows = stmt.query(params![like_pattern])?;
    let mut files = Vec::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let path: String = row.get(1)?;
        files.push((id, path));
    }

    let mut results = Vec::new();
    for (file_id, file_path) in files {
        let (src_col, dst_col) = match direction {
            Some("in")  => ("dst_file_id", "src_file_id"),
            _           => ("src_file_id", "dst_file_id"),
        };

        let sql = format!(
            "SELECT f2.path, l.type
             FROM links l
             JOIN files f2 ON f2.id = l.{dst}
             WHERE l.{src} = ?1
               AND (?2 IS NULL OR l.type = ?2)",
            src = src_col,
            dst = dst_col,
        );

        let mut stmt2 = conn.prepare(&sql)?;
        let mut rows2 = stmt2.query(params![file_id, link_type])?;
        while let Some(r2) = rows2.next()? {
            let other: String = r2.get(0)?;
            let typ: Option<String> = r2.get(1)?;
            results.push((file_path.clone(), other, typ));
        }
    }

    Ok(results)
}

/// Find all incoming links (backlinks) to files matching a pattern.
pub fn find_backlinks(conn: &Connection, pattern: &str) -> Result<Vec<(String, Option<String>)>> {
    let like_pattern = pattern.replace('*', "%");
    let mut stmt = conn.prepare(
        "SELECT f1.path, l.type
         FROM links l
         JOIN files f1 ON f1.id = l.src_file_id
         JOIN files f2 ON f2.id = l.dst_file_id
         WHERE f2.path LIKE ?1",
    )?;
    let mut rows = stmt.query(params![like_pattern])?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        let src_path: String = row.get(0)?;
        let typ: Option<String> = row.get(1)?;
        result.push((src_path, typ));
    }
    Ok(result)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_in_memory() {
        // Opening an in-memory database should apply every migration without error.
        let _conn = open(":memory:").expect("in-memory migrations should run cleanly");
    }
}
