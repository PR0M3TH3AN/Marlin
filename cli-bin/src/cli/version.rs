// src/cli/version.rs
use crate::cli::Format;
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Subcommand, Debug)]
pub enum VersionCmd {
    Diff(ArgsDiff),
}

#[derive(Args, Debug)]
pub struct ArgsDiff {
    pub file: String,
}

pub fn run(cmd: &VersionCmd, _conn: &mut Connection, _format: Format) -> anyhow::Result<()> {
    match cmd {
        VersionCmd::Diff(a) => todo!("version diff {:?}", a),
    }
}
