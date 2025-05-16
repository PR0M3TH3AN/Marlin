// src/cli/state.rs
use clap::{Subcommand, Args};
use rusqlite::Connection;
use crate::cli::Format;

#[derive(Subcommand, Debug)]
pub enum StateCmd {
    Set(ArgsSet),
    TransitionsAdd(ArgsTrans),
    Log(ArgsLog),
}

#[derive(Args, Debug)]
pub struct ArgsSet   { pub file_pattern: String, pub new_state: String }
#[derive(Args, Debug)]
pub struct ArgsTrans { pub from_state: String,   pub to_state: String }
#[derive(Args, Debug)]
pub struct ArgsLog   { pub file_pattern: String }

pub fn run(cmd: &StateCmd, _conn: &mut Connection, _format: Format) -> anyhow::Result<()> {
    match cmd {
        StateCmd::Set(a)           => todo!("state set {:?}", a),
        StateCmd::TransitionsAdd(a)=> todo!("state transitions-add {:?}", a),
        StateCmd::Log(a)           => todo!("state log {:?}", a),
    }
}
