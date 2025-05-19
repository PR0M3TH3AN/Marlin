//! libmarlin – public API surface for the Marlin core.
//!
//! Down-stream crates (`cli-bin`, `tui-bin`, tests, plugins) should depend
//! *only* on the helpers re-exported here, never on internal modules
//! directly.  That gives us room to refactor internals without breaking
//! callers.

#![deny(warnings)]

pub mod config;   // as-is
pub mod db;       // as-is
pub mod logging;  // expose the logging init helper
pub mod scan;     // as-is
pub mod utils;    // hosts determine_scan_root() & misc helpers

#[cfg(test)]
mod utils_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod scan_tests;
#[cfg(test)]
mod logging_tests;
#[cfg(test)]
mod db_tests;
#[cfg(test)]
mod facade_tests;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::{fs, path::Path};

/// Main handle for interacting with a Marlin database.
pub struct Marlin {
    #[allow(dead_code)]
    cfg:  config::Config,
    conn: Connection,
}

impl Marlin {
    /// Open using the default config (env override or XDG/CWD fallback),
    /// ensuring parent directories exist and applying migrations.
    pub fn open_default() -> Result<Self> {
        // 1) Load configuration (checks MARLIN_DB_PATH, XDG_DATA_HOME, or falls back to ./index_<hash>.db)
        let cfg = config::Config::load()?;
        // 2) Ensure the DB's parent directory exists
        if let Some(parent) = cfg.db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // 3) Open the database and run migrations
        let conn = db::open(&cfg.db_path)
            .context(format!("opening database at {}", cfg.db_path.display()))?;
        Ok(Marlin { cfg, conn })
    }

    /// Open a Marlin instance at the specified database path,
    /// creating parent directories and applying migrations.
    pub fn open_at<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = db_path.as_ref();
        // Ensure the specified DB directory exists
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Build a minimal Config so callers can still inspect cfg.db_path
        let cfg = config::Config { db_path: db_path.to_path_buf() };
        // Open the database and run migrations
        let conn = db::open(db_path)
            .context(format!("opening database at {}", db_path.display()))?;
        Ok(Marlin { cfg, conn })
    }

    /// Recursively index one or more directories.
    pub fn scan<P: AsRef<Path>>(&mut self, paths: &[P]) -> Result<usize> {
        let mut total = 0;
        for p in paths {
            total += scan::scan_directory(&mut self.conn, p.as_ref())?;
        }
        Ok(total)
    }

    /// Attach a hierarchical tag (`foo/bar`) to every _indexed_ file
    /// matching the glob.  Returns the number of files actually updated.
    pub fn tag(&mut self, pattern: &str, tag_path: &str) -> Result<usize> {
        use glob::Pattern;

        // 1) ensure tag hierarchy
        let leaf = db::ensure_tag_path(&self.conn, tag_path)?;

        // 2) collect it plus all ancestors
        let mut tag_ids = Vec::new();
        let mut cur = Some(leaf);
        while let Some(id) = cur {
            tag_ids.push(id);
            cur = self.conn.query_row(
                "SELECT parent_id FROM tags WHERE id = ?1",
                [id],
                |r| r.get::<_, Option<i64>>(0),
            )?;
        }

        // 3) pick matching files _from the DB_ (not from the FS!)
        let expanded = shellexpand::tilde(pattern).into_owned();
        let pat = Pattern::new(&expanded)
            .with_context(|| format!("Invalid glob pattern `{}`", expanded))?;

        // pull down all (id, path)
        let mut stmt_all = self.conn.prepare("SELECT id, path FROM files")?;
        let rows = stmt_all.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;

        let mut stmt_insert = self.conn.prepare(
            "INSERT OR IGNORE INTO file_tags(file_id, tag_id) VALUES (?1, ?2)",
        )?;

        let mut changed = 0;
        for row in rows {
            let (fid, path_str): (i64, String) = row?;
            let matches = if expanded.contains(std::path::MAIN_SEPARATOR) {
                // pattern includes a slash — match full path
                pat.matches(&path_str)
            } else {
                // no slash — match just the file name
                std::path::Path::new(&path_str)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| pat.matches(n))
                    .unwrap_or(false)
            };
            if !matches {
                continue;
            }

            // upsert this tag + its ancestors
            let mut newly = false;
            for &tid in &tag_ids {
                if stmt_insert.execute([fid, tid])? > 0 {
                    newly = true;
                }
            }
            if newly {
                changed += 1;
            }
        }

        Ok(changed)
    }

    /// Full‐text search over path, tags, and attrs (with fallback).
    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT f.path
              FROM files_fts
              JOIN files f ON f.rowid = files_fts.rowid
             WHERE files_fts MATCH ?1
             ORDER BY rank
            "#,
        )?;
        let mut hits = stmt
            .query_map([query], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        // graceful fallback: substring scan when no FTS hits and no `:` in query
        if hits.is_empty() && !query.contains(':') {
            hits = self.fallback_search(query)?;
        }

        Ok(hits)
    }

    /// private helper: scan `files` table + small files for a substring
    fn fallback_search(&self, term: &str) -> Result<Vec<String>> {
        let needle = term.to_lowercase();
        let mut stmt = self.conn.prepare("SELECT path FROM files")?;
        let rows = stmt.query_map([], |r| r.get(0))?;

        let mut out = Vec::new();
        for path_res in rows {
            let p: String = path_res?;  // Explicit type annotation added
            // match in the path itself?
            if p.to_lowercase().contains(&needle) {
                out.push(p.clone());
                continue;
            }
            // otherwise read small files
            if let Ok(meta) = fs::metadata(&p) {
                if meta.len() <= 65_536 {
                    if let Ok(body) = fs::read_to_string(&p) {
                        if body.to_lowercase().contains(&needle) {
                            out.push(p.clone());
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// Borrow the underlying SQLite connection (read-only).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}