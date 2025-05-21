// src/cli/task.rs
use crate::cli::Format;
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Subcommand, Debug)]
pub enum TaskCmd {
    Scan(ArgsScan),
    List(ArgsList),
}

#[derive(Args, Debug)]
pub struct ArgsScan {
    pub directory: String,
}
#[derive(Args, Debug)]
pub struct ArgsList {
    #[arg(long)]
    pub due_today: bool,
}

pub fn run(cmd: &TaskCmd, _conn: &mut Connection, _format: Format) -> anyhow::Result<()> {
    match cmd {
        TaskCmd::Scan(a) => todo!("task scan {:?}", a),
        TaskCmd::List(a) => todo!("task list {:?}", a),
    }
}
