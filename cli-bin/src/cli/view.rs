//! `marlin view …` – save & use “smart folders” (named queries).

use std::fs;

use anyhow::Result;
use clap::{Args, Subcommand};
use rusqlite::Connection;

use crate::cli::Format;   // output selector stays local
use libmarlin::db;        // ← path switched from `crate::db`

#[derive(Subcommand, Debug)]
pub enum ViewCmd {
    /// Save (or update) a view
    Save(ArgsSave),
    /// List all saved views
    List,
    /// Execute a view (print matching paths)
    Exec(ArgsExec),
}

#[derive(Args, Debug)]
pub struct ArgsSave {
    pub view_name: String,
    pub query: String,
}

#[derive(Args, Debug)]
pub struct ArgsExec {
    pub view_name: String,
}

pub fn run(cmd: &ViewCmd, conn: &mut Connection, fmt: Format) -> anyhow::Result<()> {
    match cmd {
        /* ── view save ───────────────────────────────────────────── */
        ViewCmd::Save(a) => {
            db::save_view(conn, &a.view_name, &a.query)?;
            if matches!(fmt, Format::Text) {
                println!("Saved view '{}' = {}", a.view_name, a.query);
            }
        }

        /* ── view list ───────────────────────────────────────────── */
        ViewCmd::List => {
            let views = db::list_views(conn)?;
            match fmt {
                Format::Text => {
                    for (name, q) in views {
                        println!("{name}: {q}");
                    }
                }
                Format::Json => {
                    #[cfg(feature = "json")]
                    {
                        println!("{}", serde_json::to_string(&views)?);
                    }
                }
            }
        }

        /* ── view exec ───────────────────────────────────────────── */
        ViewCmd::Exec(a) => {
            let raw = db::view_query(conn, &a.view_name)?;

            // Re-use the tiny parser from marlin search
            let fts_expr = build_fts_match(&raw);

            let mut stmt = conn.prepare(
                r#"
                SELECT f.path
                  FROM files_fts
                  JOIN files f ON f.rowid = files_fts.rowid
                 WHERE files_fts MATCH ?1
                 ORDER BY rank
                "#,
            )?;
            let mut paths: Vec<String> = stmt
                .query_map([fts_expr], |r| r.get::<_, String>(0))?
                .collect::<Result<_, _>>()?;

            /* ── NEW: graceful fallback when FTS finds nothing ───── */
            if paths.is_empty() && !raw.contains(':') {
                paths = naive_search(conn, &raw)?;
            }

            if paths.is_empty() && matches!(fmt, Format::Text) {
                eprintln!("(view '{}' has no matches)", a.view_name);
            } else {
                for p in paths {
                    println!("{p}");
                }
            }
        }
    }
    Ok(())
}

/* ─── naive substring path/content search (≤ 64 kB files) ───────── */

fn naive_search(conn: &Connection, term: &str) -> Result<Vec<String>> {
    let term_lc = term.to_lowercase();
    let mut stmt = conn.prepare("SELECT path FROM files")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;

    let mut hits = Vec::new();
    for p in rows {
        let p = p?;
        /* path match */
        if p.to_lowercase().contains(&term_lc) {
            hits.push(p);
            continue;
        }
        /* small-file content match */
        if let Ok(meta) = fs::metadata(&p) {
            if meta.len() > 64_000 {
                continue;
            }
        }
        if let Ok(content) = fs::read_to_string(&p) {
            if content.to_lowercase().contains(&term_lc) {
                hits.push(p);
            }
        }
    }
    Ok(hits)
}

/* ─── minimal copy of search-string → FTS5 translator ───────────── */

fn build_fts_match(raw_query: &str) -> String {
    use shlex;
    let mut parts = Vec::new();
    let toks = shlex::split(raw_query).unwrap_or_else(|| vec![raw_query.to_string()]);
    for tok in toks {
        if ["AND", "OR", "NOT"].contains(&tok.as_str()) {
            parts.push(tok);
        } else if let Some(tag) = tok.strip_prefix("tag:") {
            for (i, seg) in tag.split('/').filter(|s| !s.is_empty()).enumerate() {
                if i > 0 {
                    parts.push("AND".into());
                }
                parts.push(format!("tags_text:{}", escape(seg)));
            }
        } else if let Some(attr) = tok.strip_prefix("attr:") {
            let mut kv = attr.splitn(2, '=');
            let key = kv.next().unwrap();
            if let Some(val) = kv.next() {
                parts.push(format!("attrs_text:{}", escape(key)));
                parts.push("AND".into());
                parts.push(format!("attrs_text:{}", escape(val)));
            } else {
                parts.push(format!("attrs_text:{}", escape(key)));
            }
        } else {
            parts.push(escape(&tok));
        }
    }
    parts.join(" ")
}

fn escape(term: &str) -> String {
    if term.contains(|c: char| c.is_whitespace() || "-:()\"".contains(c))
        || ["AND", "OR", "NOT", "NEAR"].contains(&term.to_uppercase().as_str())
    {
        format!("\"{}\"", term.replace('"', "\"\""))
    } else {
        term.to_string()
    }
}
