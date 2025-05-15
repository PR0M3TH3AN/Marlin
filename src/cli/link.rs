// src/cli/link.rs
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
        LinkCmd::Add(args)       => todo!("link add {:?}", args),
        LinkCmd::Rm(args)        => todo!("link rm {:?}", args),
        LinkCmd::List(args)      => todo!("link list {:?}", args),
        LinkCmd::Backlinks(args) => todo!("link backlinks {:?}", args),
    }
}
