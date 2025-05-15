// src/cli/view.rs
use clap::{Subcommand, Args};
use rusqlite::Connection;
use crate::cli::Format;

#[derive(Subcommand, Debug)]
pub enum ViewCmd {
    Save(ArgsSave),
    List,
    Exec(ArgsExec),
}

#[derive(Args, Debug)]
pub struct ArgsSave  { pub view_name: String, pub query: String }
#[derive(Args, Debug)]
pub struct ArgsExec  { pub view_name: String }

pub fn run(cmd: &ViewCmd, conn: &mut Connection, format: Format) -> anyhow::Result<()> {
    match cmd {
        ViewCmd::Save(a) => todo!("view save {:?}", a),
        ViewCmd::List   => todo!("view list"),
        ViewCmd::Exec(a)=> todo!("view exec {:?}", a),
    }
}
