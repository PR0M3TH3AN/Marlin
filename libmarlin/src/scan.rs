// src/scan.rs

use std::fs;
use std::path::Path;

use crate::utils::to_db_path;

use anyhow::Result;
use rusqlite::{params, Connection};
use tracing::{debug, info};
use walkdir::WalkDir;

/// Recursively walk `root` and upsert file metadata.
/// Triggers keep the FTS table in sync.
pub fn scan_directory(conn: &mut Connection, root: &Path) -> Result<usize> {
    // Begin a transaction so we batch many inserts/updates together
    let tx = conn.transaction()?;

    // Prepare the upsert statement once
    let mut stmt = tx.prepare(
        r#"
        INSERT INTO files(path, size, mtime)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(path) DO UPDATE
            SET size  = excluded.size,
                mtime = excluded.mtime
        "#,
    )?;

    let mut count = 0usize;

    // Walk the directory recursively
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip the database file and its WAL/SHM siblings
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".db") || name.ends_with("-wal") || name.ends_with("-shm") {
                continue;
            }
        }

        // Gather file metadata
        let meta = fs::metadata(path)?;
        let size = meta.len() as i64;
        let mtime = meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        // Execute the upsert
        let path_str = to_db_path(path);
        stmt.execute(params![path_str, size, mtime])?;
        count += 1;

        debug!(file = %path_str, "indexed");
    }

    // Finalize and commit
    drop(stmt);
    tx.commit()?;

    info!(indexed = count, "scan complete");
    Ok(count)
}
