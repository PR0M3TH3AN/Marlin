use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};

const MIGRATIONS_SQL: &str = include_str!("migrations.sql");

/// Open (or create) the SQLite database and run embedded migrations.
pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let mut conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(MIGRATIONS_SQL)?;
    Ok(conn)
}

/// Ensure a tag exists, returning its id.
pub fn ensure_tag(conn: &Connection, tag: &str) -> Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO tags(name) VALUES (?1)",
        params![tag],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM tags WHERE name = ?1",
        params![tag],
        |row| row.get(0),
    )?;
    Ok(id)
}
