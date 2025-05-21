//! tests/util.rs
//! Small helpers shared across integration tests.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
/// Absolute path to the freshly-built `marlin` binary.
pub fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_marlin"))
}

/// Build a `Command` for `marlin` whose `MARLIN_DB_PATH` is
/// `<tmp>/index.db`.
///
/// Each call yields a brand-new `Command`, so callers can freely add
/// arguments, change the working directory, etc., without affecting
/// other invocations.
pub fn marlin(tmp: &TempDir) -> Command {
    let db_path: &Path = &tmp.path().join("index.db");
    let mut cmd = Command::new(bin());
    cmd.env("MARLIN_DB_PATH", db_path);
    cmd
}
