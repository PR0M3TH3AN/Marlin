//! `marlin coll …` – named collections of files (simple “playlists”).

use clap::{Args, Subcommand};
use rusqlite::Connection;

use crate::cli::Format; // local enum for text / json output
use libmarlin::db; // core DB helpers from the library crate

#[derive(Subcommand, Debug)]
pub enum CollCmd {
    /// Create an empty collection
    Create(CreateArgs),
    /// Add files (glob) to a collection
    Add(AddArgs),
    /// List files inside a collection
    List(ListArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    pub name: String,
    pub file_pattern: String,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    pub name: String,
}

/// Look-up an existing collection **without** implicitly creating it.
///
/// Returns the collection ID or an error if it doesn’t exist.
fn lookup_collection_id(conn: &Connection, name: &str) -> anyhow::Result<i64> {
    conn.query_row("SELECT id FROM collections WHERE name = ?1", [name], |r| {
        r.get(0)
    })
    .map_err(|_| anyhow::anyhow!("collection not found: {}", name))
}

pub fn run(cmd: &CollCmd, conn: &mut Connection, fmt: Format) -> anyhow::Result<()> {
    match cmd {
        /* ── coll create ──────────────────────────────────────────── */
        CollCmd::Create(a) => {
            db::ensure_collection(conn, &a.name)?;
            if matches!(fmt, Format::Text) {
                println!("Created collection '{}'", a.name);
            }
        }

        /* ── coll add ─────────────────────────────────────────────── */
        CollCmd::Add(a) => {
            // Fail if the target collection does not yet exist
            let coll_id = lookup_collection_id(conn, &a.name)?;

            let like = a.file_pattern.replace('*', "%");
            let mut stmt = conn.prepare("SELECT id FROM files WHERE path LIKE ?1")?;
            let ids: Vec<i64> = stmt
                .query_map([&like], |r| r.get::<_, i64>(0))?
                .collect::<Result<_, _>>()?;

            for fid in &ids {
                db::add_file_to_collection(conn, coll_id, *fid)?;
            }

            match fmt {
                Format::Text => println!("Added {} file(s) → '{}'", ids.len(), a.name),
                Format::Json => {
                    #[cfg(feature = "json")]
                    {
                        println!("{{\"collection\":\"{}\",\"added\":{}}}", a.name, ids.len());
                    }
                }
            }
        }

        /* ── coll list ────────────────────────────────────────────── */
        CollCmd::List(a) => {
            let files = db::list_collection(conn, &a.name)?;
            match fmt {
                Format::Text => {
                    for f in files {
                        println!("{f}");
                    }
                }
                Format::Json => {
                    #[cfg(feature = "json")]
                    {
                        println!("{}", serde_json::to_string(&files)?);
                    }
                }
            }
        }
    }
    Ok(())
}
