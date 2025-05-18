//! libmarlin – public API surface for the Marlin core.
//!
//! Down-stream crates (`cli-bin`, `tui-bin`, tests, plugins) should depend
//! *only* on the helpers re-exported here, never on internal modules
//! directly.  That gives us room to refactor internals without breaking
//! callers.

#![deny(warnings)]

pub mod config;   // moved as-is
pub mod db;       // moved as-is
pub mod logging;  // expose the logging init helper
pub mod scan;     // moved as-is
pub mod utils;    // hosts determine_scan_root() & misc helpers

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use walkdir::WalkDir;

/// Primary façade – open a workspace then call helper methods.
///
/// Most methods simply wrap what the CLI used to do directly; more will be
/// filled in sprint-by-sprint.
pub struct Marlin {
    #[allow(dead_code)]
    cfg:  config::Config,
    conn: Connection,
}

impl Marlin {
    /// Load configuration from env / workspace and open (or create) the DB.
    pub fn open_default() -> Result<Self> {
        let cfg  = config::Config::load()?;
        let conn = db::open(&cfg.db_path)?;
        Ok(Self { cfg, conn })
    }

    /// Open an explicit DB path – handy for tests or headless tools.
    pub fn open_at<P: AsRef<Path>>(path: P) -> Result<Self> {
        let cfg  = config::Config { db_path: path.as_ref().to_path_buf() };
        let conn = db::open(&cfg.db_path)?;
        Ok(Self { cfg, conn })
    }

    /// Recursively index one or more directories.
    pub fn scan<P: AsRef<Path>>(&mut self, paths: &[P]) -> Result<usize> {
        let mut total = 0usize;
        for p in paths {
            total += scan::scan_directory(&mut self.conn, p.as_ref())?;
        }
        Ok(total)
    }

    /// Attach a hierarchical tag (`foo/bar`) to every file that matches the
    /// glob pattern. Returns the number of files that actually got updated.
    pub fn tag(&mut self, pattern: &str, tag_path: &str) -> Result<usize> {
        use glob::Pattern;

        // 1) ensure tag hierarchy exists
        let leaf_tag_id = db::ensure_tag_path(&self.conn, tag_path)?;

        // 2) collect leaf + ancestors
        let mut tag_ids = Vec::new();
        let mut current = Some(leaf_tag_id);
        while let Some(id) = current {
            tag_ids.push(id);
            current = self.conn.query_row(
                "SELECT parent_id FROM tags WHERE id=?1",
                [id],
                |r| r.get::<_, Option<i64>>(0),
            )?;
        }

        // 3) walk the file tree and upsert `file_tags`
        let expanded = shellexpand::tilde(pattern).into_owned();
        let pat      = Pattern::new(&expanded)
            .with_context(|| format!("Invalid glob pattern `{expanded}`"))?;
        let root     = utils::determine_scan_root(&expanded);

        let mut stmt_file   = self.conn.prepare("SELECT id FROM files WHERE path=?1")?;
        let mut stmt_insert = self.conn.prepare(
            "INSERT OR IGNORE INTO file_tags(file_id, tag_id) VALUES (?1, ?2)",
        )?;

        let mut changed = 0usize;
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
                        if stmt_insert.execute([fid, tid])? > 0 { newly = true; }
                    }
                    if newly { changed += 1; }
                }
                Err(_) => { /* ignore non‐indexed files */ }
            }
        }

        Ok(changed)
    }

    /// FTS5 search → list of matching paths.
    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT path FROM files_fts WHERE files_fts MATCH ?1 ORDER BY rank",
        )?;
        let rows = stmt.query_map([query], |r| r.get::<_, String>(0))?
                       .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Borrow the underlying SQLite connection (read-only).
    pub fn conn(&self) -> &Connection { &self.conn }
}
