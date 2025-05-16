//! End-to-end smoke-tests for the marlin binary.
//!
//! Run with `cargo test --test e2e` or let CI invoke `cargo test`.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::{fs, path::PathBuf, process::Command};
use tempfile::tempdir;

/// Absolute path to the `marlin` binary Cargo just built for this test run.
fn marlin_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_marlin"))
}

fn spawn_demo_tree(root: &PathBuf) {
    fs::create_dir_all(root.join("Projects/Alpha")).unwrap();
    fs::create_dir_all(root.join("Projects/Beta")).unwrap();
    fs::create_dir_all(root.join("Projects/Gamma")).unwrap();
    fs::create_dir_all(root.join("Logs")).unwrap();
    fs::create_dir_all(root.join("Reports")).unwrap();

    fs::write(root.join("Projects/Alpha/draft1.md"), "- [ ] TODO foo\n").unwrap();
    fs::write(root.join("Projects/Alpha/draft2.md"), "- [x] TODO foo\n").unwrap();
    fs::write(root.join("Projects/Beta/final.md"),   "done\n").unwrap();
    fs::write(root.join("Projects/Gamma/TODO.txt"),  "TODO bar\n").unwrap();
    fs::write(root.join("Logs/app.log"),             "ERROR omg\n").unwrap();
    fs::write(root.join("Reports/Q1.pdf"),           "PDF\n").unwrap();
}

fn run(cmd: &mut Command) -> assert_cmd::assert::Assert {
    cmd.assert().success()
}

#[test]
fn full_cli_flow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. sandbox
    let tmp      = tempdir()?;
    let demo_dir = tmp.path().join("marlin_demo");
    spawn_demo_tree(&demo_dir);

    // 2. init  (auto-scan cwd)
    run(Command::new(marlin_bin())
        .current_dir(&demo_dir)
        .arg("init"));

    // 3. tag & attr
    run(Command::new(marlin_bin())
        .arg("tag")
        .arg(format!("{}/Projects/**/*.md", demo_dir.display()))
        .arg("project/md"));

    run(Command::new(marlin_bin())
        .arg("attr")
        .arg("set")
        .arg(format!("{}/Reports/*.pdf", demo_dir.display()))
        .arg("reviewed")
        .arg("yes"));

    // 4. search expectations
    Command::new(marlin_bin())
        .arg("search")
        .arg("TODO")
        .assert()
        .stdout(predicate::str::contains("TODO.txt"));

    Command::new(marlin_bin())
        .arg("search")
        .arg("attr:reviewed=yes")
        .assert()
        .stdout(predicate::str::contains("Q1.pdf"));

    // 5. link & backlinks
    let foo = demo_dir.join("foo.txt");
    let bar = demo_dir.join("bar.txt");
    fs::write(&foo, "")?;
    fs::write(&bar, "")?;
    run(Command::new(marlin_bin()).arg("scan").arg(&demo_dir));
    run(Command::new(marlin_bin())
        .arg("link").arg("add")
        .arg(&foo).arg(&bar));
    Command::new(marlin_bin())
        .arg("link").arg("backlinks").arg(&bar)
        .assert()
        .stdout(predicate::str::contains("foo.txt"));

    // 6. backup / restore round-trip
    let backup_path = String::from_utf8(
        Command::new(marlin_bin()).arg("backup").output()?.stdout
    )?;
    let backup_file = backup_path.split_whitespace().last().unwrap();

    // wipe DB file
    std::fs::remove_file(dirs::data_dir().unwrap().join("marlin/index.db"))?;
    run(Command::new(marlin_bin()).arg("restore").arg(backup_file));

    // sanity: search still works
    Command::new(marlin_bin())
        .arg("search").arg("TODO")
        .assert()
        .stdout(predicate::str::contains("TODO.txt"));

    Ok(())
}
