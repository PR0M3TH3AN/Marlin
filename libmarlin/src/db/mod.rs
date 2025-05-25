//! Central DB helper – connection bootstrap, migrations **and** most
//! data-access helpers (tags, links, collections, saved views, …).

mod database;
pub use database::{Database, IndexOptions};

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::{
    backup::{Backup, StepResult},
    params, Connection, OpenFlags, OptionalExtension, TransactionBehavior,
};
use std::result::Result as StdResult;
use tracing::{debug, info, warn};

use crate::utils::to_db_path;

/* ─── schema version ───────────────────────────────────────────────── */

/// Current library schema version.
pub const SCHEMA_VERSION: i32 = MIGRATIONS.len() as i32;

/* ─── embedded migrations ─────────────────────────────────────────── */

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_initial_schema.sql",
        include_str!("migrations/0001_initial_schema.sql"),
    ),
    (
        "0002_update_fts_and_triggers.sql",
        include_str!("migrations/0002_update_fts_and_triggers.sql"),
    ),
    (
        "0003_create_links_collections_views.sql",
        include_str!("migrations/0003_create_links_collections_views.sql"),
    ),
    (
        "0004_fix_hierarchical_tags_fts.sql",
        include_str!("migrations/0004_fix_hierarchical_tags_fts.sql"),
    ),
    (
        "0005_add_dirty_table.sql",
        include_str!("migrations/0005_add_dirty_table.sql"),
    ),
    (
        "0006_drop_tags_canonical_id.sql",
        include_str!("migrations/0006_drop_tags_canonical_id.sql"),
    ),
    (
        "0007_fix_rename_trigger.sql",
        include_str!("migrations/0007_fix_rename_trigger.sql"),
    ),
];

/* ─── schema helpers ─────────────────────────────────────────────── */

/// Fetch the highest version recorded in the `schema_version` table.
pub fn current_schema_version(conn: &Connection) -> Result<i32> {
    let version: i32 = conn.query_row(
        "SELECT IFNULL(MAX(version), 0) FROM schema_version",
        [],
        |r| r.get(0),
    )?;
    Ok(version)
}

/* ─── connection bootstrap ────────────────────────────────────────── */

pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let db_path_ref = db_path.as_ref();
    let mut conn = Connection::open(db_path_ref)
        .with_context(|| format!("failed to open DB at {}", db_path_ref.display()))?;

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Wait up to 30 s for a competing writer before giving up
    conn.busy_timeout(std::time::Duration::from_secs(30))?;

    apply_migrations(&mut conn)?;
    Ok(conn)
}

/* ─── migration runner ────────────────────────────────────────────── */

pub(crate) fn apply_migrations(conn: &mut Connection) -> Result<()> {
    // Ensure schema_version bookkeeping table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
             version     INTEGER PRIMARY KEY,
             applied_on  TEXT NOT NULL
         );",
    )?;

    // Legacy patch – ignore errors if column already exists
    let _ = conn.execute("ALTER TABLE schema_version ADD COLUMN applied_on TEXT", []);

    // Grab the write-lock up-front so migrations can run uninterrupted
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

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
        tx.execute_batch(sql)
            .with_context(|| format!("could not apply migration {}", fname))?;

        tx.execute(
            "INSERT INTO schema_version (version, applied_on) VALUES (?1, ?2)",
            params![version, Local::now().to_rfc3339()],
        )?;
    }

    tx.commit()?;

    // sanity – warn if any embedded migration got skipped
    let mut missing = Vec::new();
    for (fname, _) in MIGRATIONS {
        let v: i64 = fname.split('_').next().unwrap().parse().unwrap();
        let ok: bool = conn
            .query_row(
                "SELECT 1 FROM schema_version WHERE version = ?1",
                [v],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !ok {
            missing.push(v);
        }
    }
    if !missing.is_empty() {
        warn!("migrations not applied: {:?}", missing);
    }

    let current = current_schema_version(conn)?;
    if current != SCHEMA_VERSION {
        anyhow::bail!(
            "database schema version {} does not match library version {}",
            current,
            SCHEMA_VERSION
        );
    }

    Ok(())
}

/* ─── tag helpers ─────────────────────────────────────────────────── */

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
            |r| r.get(0),
        )?;
        parent = Some(id);
    }
    parent.ok_or_else(|| anyhow::anyhow!("empty tag path"))
}

pub fn file_id(conn: &Connection, path: &str) -> Result<i64> {
    let path = to_db_path(path);
    conn.query_row(
        "SELECT id FROM files WHERE path = ?1",
        [path.clone()],
        |r| r.get(0),
    )
    .map_err(|_| anyhow::anyhow!("file not indexed: {}", path))
}

/* ─── attributes ──────────────────────────────────────────────────── */

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

/* ─── links ───────────────────────────────────────────────────────── */

pub fn add_link(
    conn: &Connection,
    src_file_id: i64,
    dst_file_id: i64,
    link_type: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO links(src_file_id, dst_file_id, type)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(src_file_id, dst_file_id, type) DO NOTHING",
        params![src_file_id, dst_file_id, link_type],
    )?;
    Ok(())
}

pub fn remove_link(
    conn: &Connection,
    src_file_id: i64,
    dst_file_id: i64,
    link_type: Option<&str>,
) -> Result<()> {
    conn.execute(
        "DELETE FROM links
         WHERE src_file_id = ?1
           AND dst_file_id = ?2
           AND (type IS ?3 OR type = ?3)",
        params![src_file_id, dst_file_id, link_type],
    )?;
    Ok(())
}

pub fn list_links(
    conn: &Connection,
    pattern: &str,
    direction: Option<&str>,
    link_type: Option<&str>,
) -> Result<Vec<(String, String, Option<String>)>> {
    let like_pattern = pattern.replace('*', "%");

    // Files matching pattern
    let mut stmt = conn.prepare("SELECT id, path FROM files WHERE path LIKE ?1")?;
    let rows = stmt
        .query_map(params![like_pattern], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<StdResult<Vec<_>, _>>()?;

    let mut out = Vec::new();
    for (fid, fpath) in rows {
        let (src_col, dst_col) = match direction {
            Some("in") => ("dst_file_id", "src_file_id"),
            _ => ("src_file_id", "dst_file_id"),
        };

        let sql = format!(
            "SELECT f2.path, l.type
               FROM links l
               JOIN files f2 ON f2.id = l.{dst_col}
              WHERE l.{src_col} = ?1
                AND (?2 IS NULL OR l.type = ?2)",
        );

        let mut stmt2 = conn.prepare(&sql)?;
        let links = stmt2
            .query_map(params![fid, link_type], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })?
            .collect::<StdResult<Vec<_>, _>>()?;

        for (other, typ) in links {
            out.push((fpath.clone(), other, typ));
        }
    }
    Ok(out)
}

pub fn find_backlinks(conn: &Connection, pattern: &str) -> Result<Vec<(String, Option<String>)>> {
    let like = pattern.replace('*', "%");

    let mut stmt = conn.prepare(
        "SELECT f1.path, l.type
           FROM links l
           JOIN files f1 ON f1.id = l.src_file_id
           JOIN files f2 ON f2.id = l.dst_file_id
          WHERE f2.path LIKE ?1",
    )?;

    let rows = stmt.query_map([like], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
    })?;

    let out = rows.collect::<StdResult<Vec<_>, _>>()?;
    Ok(out)
}

/* ─── collections helpers ────────────────────────────────────────── */

pub fn ensure_collection(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO collections(name) VALUES (?1)",
        params![name],
    )?;
    conn.query_row(
        "SELECT id FROM collections WHERE name = ?1",
        params![name],
        |r| r.get(0),
    )
    .context("collection lookup failed")
}

pub fn add_file_to_collection(conn: &Connection, coll_id: i64, file_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO collection_files(collection_id, file_id)
         VALUES (?1, ?2)",
        params![coll_id, file_id],
    )?;
    Ok(())
}

pub fn list_collection(conn: &Connection, name: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        r#"SELECT f.path
            FROM collections        c
            JOIN collection_files cf ON cf.collection_id = c.id
            JOIN files            f  ON f.id            = cf.file_id
           WHERE c.name = ?1
           ORDER BY f.path"#,
    )?;

    let rows = stmt.query_map([name], |r| r.get::<_, String>(0))?;
    let list = rows.collect::<StdResult<Vec<_>, _>>()?;
    Ok(list)
}

/* ─── saved views (smart folders) ───────────────────────────────── */

pub fn save_view(conn: &Connection, name: &str, query: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO views(name, query)
         VALUES (?1, ?2)
         ON CONFLICT(name) DO UPDATE SET query = excluded.query",
        params![name, query],
    )?;
    Ok(())
}

pub fn list_views(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT name, query FROM views ORDER BY name")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let list = rows.collect::<StdResult<Vec<_>, _>>()?;
    Ok(list)
}

pub fn view_query(conn: &Connection, name: &str) -> Result<String> {
    conn.query_row("SELECT query FROM views WHERE name = ?1", [name], |r| {
        r.get::<_, String>(0)
    })
    .context(format!("no view called '{}'", name))
}

/* ─── dirty‐scan helpers ─────────────────────────────────────────── */

/// Mark a file as “dirty” so it’ll be picked up by `scan_dirty`.
pub fn mark_dirty(conn: &Connection, file_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO file_changes(file_id, marked_at)
         VALUES (?1, strftime('%s','now'))",
        params![file_id],
    )?;
    Ok(())
}

/// Take and clear all dirty file IDs for incremental re-scan.
pub fn take_dirty(conn: &Connection) -> Result<Vec<i64>> {
    let mut ids = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT file_id FROM file_changes")?;
        for row in stmt.query_map([], |r| r.get(0))? {
            ids.push(row?);
        }
    }
    conn.execute("DELETE FROM file_changes", [])?;
    Ok(ids)
}

/* ─── rename helpers ────────────────────────────────────────────── */

pub fn update_file_path(conn: &Connection, old_path: &str, new_path: &str) -> Result<()> {
    let old_path = to_db_path(old_path);
    let new_path = to_db_path(new_path);

    let file_id: i64 =
        conn.query_row("SELECT id FROM files WHERE path = ?1", [&old_path], |r| {
            r.get(0)
        })?;
    conn.execute(
        "UPDATE files SET path = ?1 WHERE id = ?2",
        params![new_path, file_id],
    )?;
    mark_dirty(conn, file_id)?;
    Ok(())
}

pub fn rename_directory(conn: &mut Connection, old_dir: &str, new_dir: &str) -> Result<()> {
    let old_dir = to_db_path(old_dir);
    let new_dir = to_db_path(new_dir);
    let like_pattern = format!("{}/%", old_dir.trim_end_matches('/'));
    let ids = {
        let mut stmt = conn.prepare("SELECT id FROM files WHERE path LIKE ?1")?;
        let rows = stmt.query_map([&like_pattern], |r| r.get::<_, i64>(0))?;
        rows.collect::<StdResult<Vec<_>, _>>()?
    };
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE files SET path = REPLACE(path, ?1, ?2) WHERE path LIKE ?3",
        params![old_dir, new_dir, like_pattern],
    )?;
    for fid in ids {
        mark_dirty(&tx, fid)?;
    }
    tx.commit()?;
    Ok(())
}

/* ─── backup / restore helpers ────────────────────────────────────── */

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

/* ─── tests ───────────────────────────────────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_in_memory() {
        open(":memory:").expect("all migrations apply");
    }
}
