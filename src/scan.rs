use std::fs;
use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};
use tracing::{debug, info};
use walkdir::WalkDir;

/// Recursively walk `root` and upsert file metadata.
pub fn scan_directory(conn: &Connection, root: &Path) -> Result<usize> {
    let tx = conn.transaction()?;
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
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file())
    {
        let meta = fs::metadata(entry.path())?;
        let size = meta.len() as i64;
        let mtime = meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let path_str = entry.path().to_string_lossy();
        stmt.execute(params![path_str, size, mtime])?;
        count += 1;
        debug!(file = %path_str, "indexed");
    }

    tx.commit()?;
    info!(indexed = count, "scan complete");
    Ok(count)
}
