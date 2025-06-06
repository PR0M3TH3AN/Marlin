// src/cli/remind.rs
use crate::cli::Format;
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Subcommand, Debug)]
pub enum RemindCmd {
    Set(ArgsSet),
}

#[derive(Args, Debug)]
pub struct ArgsSet {
    pub file_pattern: String,
    pub timestamp: String,
    pub message: String,
}

pub fn run(cmd: &RemindCmd, _conn: &mut Connection, _format: Format) -> anyhow::Result<()> {
    match cmd {
        RemindCmd::Set(a) => todo!("remind set {:?}", a),
    }
}
