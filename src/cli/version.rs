// src/cli/version.rs
use clap::{Subcommand, Args};
use rusqlite::Connection;
use crate::cli::Format;

#[derive(Subcommand, Debug)]
pub enum VersionCmd {
    Diff(ArgsDiff),
}

#[derive(Args, Debug)]
pub struct ArgsDiff { pub file: String }

pub fn run(cmd: &VersionCmd, conn: &mut Connection, format: Format) -> anyhow::Result<()> {
    match cmd {
        VersionCmd::Diff(a) => todo!("version diff {:?}", a),
    }
}
