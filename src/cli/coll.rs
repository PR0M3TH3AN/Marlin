// src/cli/coll.rs
use clap::{Subcommand, Args};
use rusqlite::Connection;
use crate::cli::Format;

#[derive(Subcommand, Debug)]
pub enum CollCmd {
    Create(CreateArgs),
    Add   (AddArgs),
    List  (ListArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs { pub name: String }
#[derive(Args, Debug)]
pub struct AddArgs    { pub name: String, pub file_pattern: String }
#[derive(Args, Debug)]
pub struct ListArgs   { pub name: String }

pub fn run(cmd: &CollCmd, _conn: &mut Connection, _format: Format) -> anyhow::Result<()> {
    match cmd {
        CollCmd::Create(a) => todo!("coll create {:?}", a),
        CollCmd::Add(a)    => todo!("coll add {:?}", a),
        CollCmd::List(a)   => todo!("coll list {:?}", a),
    }
}
