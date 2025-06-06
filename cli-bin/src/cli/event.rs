// src/cli/event.rs
use crate::cli::Format;
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Subcommand, Debug)]
pub enum EventCmd {
    Add(ArgsAdd),
    Timeline,
}

#[derive(Args, Debug)]
pub struct ArgsAdd {
    pub file: String,
    pub date: String,
    pub description: String,
}

pub fn run(cmd: &EventCmd, _conn: &mut Connection, _format: Format) -> anyhow::Result<()> {
    match cmd {
        EventCmd::Add(a) => todo!("event add {:?}", a),
        EventCmd::Timeline => todo!("event timeline"),
    }
}
