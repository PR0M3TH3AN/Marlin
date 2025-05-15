// src/cli/remind.rs
use clap::{Subcommand, Args};
use rusqlite::Connection;
use crate::cli::Format;

#[derive(Subcommand, Debug)]
pub enum RemindCmd {
    Set(ArgsSet),
}

#[derive(Args, Debug)]
pub struct ArgsSet {
    pub file_pattern: String,
    pub timestamp:    String,
    pub message:      String,
}

pub fn run(cmd: &RemindCmd, conn: &mut Connection, format: Format) -> anyhow::Result<()> {
    match cmd {
        RemindCmd::Set(a) => todo!("remind set {:?}", a),
    }
}
