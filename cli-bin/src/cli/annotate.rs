// src/cli/annotate.rs
use crate::cli::Format;
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Subcommand, Debug)]
pub enum AnnotateCmd {
    Add(ArgsAdd),
    List(ArgsList),
}

#[derive(Args, Debug)]
pub struct ArgsAdd {
    pub file: String,
    pub note: String,
    #[arg(long)]
    pub range: Option<String>,
    #[arg(long)]
    pub highlight: bool,
}

#[derive(Args, Debug)]
pub struct ArgsList {
    pub file_pattern: String,
}

pub fn run(cmd: &AnnotateCmd, _conn: &mut Connection, _format: Format) -> anyhow::Result<()> {
    match cmd {
        AnnotateCmd::Add(a) => todo!("annotate add {:?}", a),
        AnnotateCmd::List(a) => todo!("annotate list {:?}", a),
    }
}
