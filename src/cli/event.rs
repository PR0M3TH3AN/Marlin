// src/cli/event.rs
use clap::{Subcommand, Args};
use rusqlite::Connection;
use crate::cli::Format;

#[derive(Subcommand, Debug)]
pub enum EventCmd {
    Add     (ArgsAdd),
    Timeline,
}

#[derive(Args, Debug)]
pub struct ArgsAdd {
    pub file: String,
    pub date: String,
    pub description: String,
}

pub fn run(cmd: &EventCmd, conn: &mut Connection, format: Format) -> anyhow::Result<()> {
    match cmd {
        EventCmd::Add(a)      => todo!("event add {:?}", a),
        EventCmd::Timeline    => todo!("event timeline"),
    }
}
