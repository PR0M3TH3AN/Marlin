// src/cli/link.rs

use crate::db;
use clap::{Subcommand, Args};
use rusqlite::Connection;
use crate::cli::Format;

#[derive(Subcommand, Debug)]
pub enum LinkCmd {
    Add(LinkArgs),
    Rm (LinkArgs),
    List(ListArgs),
    Backlinks(BacklinksArgs),
}

#[derive(Args, Debug)]
pub struct LinkArgs {
    pub from: String,
    pub to:   String,
    #[arg(long)]
    pub r#type: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    pub pattern: String,
    #[arg(long)]
    pub direction: Option<String>,
    #[arg(long)]
    pub r#type: Option<String>,
}

#[derive(Args, Debug)]
pub struct BacklinksArgs {
    pub pattern: String,
}

pub fn run(cmd: &LinkCmd, conn: &mut Connection, format: Format) -> anyhow::Result<()> {
    match cmd {
        LinkCmd::Add(args) => {
            let src_id = db::file_id(conn, &args.from)?;
            let dst_id = db::file_id(conn, &args.to)?;
            db::add_link(conn, src_id, dst_id, args.r#type.as_deref())?;
            match format {
                Format::Text => {
                    if let Some(t) = &args.r#type {
                        println!("Linked '{}' → '{}' [type='{}']", args.from, args.to, t);
                    } else {
                        println!("Linked '{}' → '{}'", args.from, args.to);
                    }
                }
                Format::Json => {
                    let typ = args
                        .r#type
                        .as_ref()
                        .map(|s| format!("\"{}\"", s))
                        .unwrap_or_else(|| "null".into());
                    println!(
                        "{{\"from\":\"{}\",\"to\":\"{}\",\"type\":{}}}",
                        args.from, args.to, typ
                    );
                }
            }
        }
        LinkCmd::Rm(args) => {
            let src_id = db::file_id(conn, &args.from)?;
            let dst_id = db::file_id(conn, &args.to)?;
            db::remove_link(conn, src_id, dst_id, args.r#type.as_deref())?;
            match format {
                Format::Text => {
                    if let Some(t) = &args.r#type {
                        println!("Removed link '{}' → '{}' [type='{}']", args.from, args.to, t);
                    } else {
                        println!("Removed link '{}' → '{}'", args.from, args.to);
                    }
                }
                Format::Json => {
                    let typ = args
                        .r#type
                        .as_ref()
                        .map(|s| format!("\"{}\"", s))
                        .unwrap_or_else(|| "null".into());
                    println!(
                        "{{\"from\":\"{}\",\"to\":\"{}\",\"type\":{}}}",
                        args.from, args.to, typ
                    );
                }
            }
        }
        LinkCmd::List(args) => {
            let results = db::list_links(
                conn,
                &args.pattern,
                args.direction.as_deref(),
                args.r#type.as_deref(),
            )?;
            match format {
                Format::Json => {
                    let items: Vec<String> = results
                        .into_iter()
                        .map(|(src, dst, t)| {
                            let typ = t
                                .as_ref()
                                .map(|s| format!("\"{}\"", s))
                                .unwrap_or_else(|| "null".into());
                            format!(
                                "{{\"from\":\"{}\",\"to\":\"{}\",\"type\":{}}}",
                                src, dst, typ
                            )
                        })
                        .collect();
                    println!("[{}]", items.join(","));
                }
                Format::Text => {
                    for (src, dst, t) in results {
                        if let Some(t) = t {
                            println!("{} → {} [type='{}']", src, dst, t);
                        } else {
                            println!("{} → {}", src, dst);
                        }
                    }
                }
            }
        }
        LinkCmd::Backlinks(args) => {
            let results = db::find_backlinks(conn, &args.pattern)?;
            match format {
                Format::Json => {
                    let items: Vec<String> = results
                        .into_iter()
                        .map(|(src, t)| {
                            let typ = t
                                .as_ref()
                                .map(|s| format!("\"{}\"", s))
                                .unwrap_or_else(|| "null".into());
                            format!("{{\"from\":\"{}\",\"type\":{}}}", src, typ)
                        })
                        .collect();
                    println!("[{}]", items.join(","));
                }
                Format::Text => {
                    for (src, t) in results {
                        if let Some(t) = t {
                            println!("{} [type='{}']", src, t);
                        } else {
                            println!("{}", src);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
