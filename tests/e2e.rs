//! End-to-end “happy path” smoke-tests for the `marlin` binary.
//!
//! Run with `cargo test --test e2e` (CI does) or `cargo test`.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::{fs, path::PathBuf, process::Command};
use tempfile::tempdir;

/// Absolute path to the freshly-built `marlin` binary.
fn marlin_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_marlin"))
}

/// Create the demo directory structure and seed files.
fn spawn_demo_tree(root: &PathBuf) {
    fs::create_dir_all(root.join("Projects/Alpha")).unwrap();
    fs::create_dir_all(root.join("Projects/Beta")).unwrap();
    fs::create_dir_all(root.join("Projects/Gamma")).unwrap();
    fs::create_dir_all(root.join("Logs")).unwrap();
    fs::create_dir_all(root.join("Reports")).unwrap();

    fs::write(root.join("Projects/Alpha/draft1.md"), "- [ ] TODO foo\n").unwrap();
    fs::write(root.join("Projects/Alpha/draft2.md"), "- [x] TODO foo\n").unwrap();
    fs::write(root.join("Projects/Beta/final.md"), "done\n").unwrap();
    fs::write(root.join("Projects/Gamma/TODO.txt"), "TODO bar\n").unwrap();
    fs::write(root.join("Logs/app.log"),           "ERROR omg\n").unwrap();
    fs::write(root.join("Reports/Q1.pdf"),         "PDF\n").unwrap();
}

/// Shorthand for “run and must succeed”.
fn ok(cmd: &mut Command) -> assert_cmd::assert::Assert {
    cmd.assert().success()
}

#[test]
fn full_cli_flow() -> Result<(), Box<dyn std::error::Error>> {
    /* ── 1 ░ sandbox ───────────────────────────────────────────── */

    let tmp      = tempdir()?;                 // wiped on drop
    let demo_dir = tmp.path().join("marlin_demo");
    spawn_demo_tree(&demo_dir);

    let db_path = demo_dir.join("index.db");

    // Helper to spawn a fresh `marlin` Command with the DB env-var set
    let marlin = || {
        let mut c = Command::new(marlin_bin());
        c.env("MARLIN_DB_PATH", &db_path);
        c
    };

    /* ── 2 ░ init ( auto-scan cwd ) ───────────────────────────── */

    ok(marlin()
        .current_dir(&demo_dir)
        .arg("init"));

    /* ── 3 ░ tag & attr demos ─────────────────────────────────── */

    ok(marlin()
        .arg("tag")
        .arg(format!("{}/Projects/**/*.md", demo_dir.display()))
        .arg("project/md"));

    ok(marlin()
        .arg("attr")
        .arg("set")
        .arg(format!("{}/Reports/*.pdf", demo_dir.display()))
        .arg("reviewed")
        .arg("yes"));

    /* ── 4 ░ quick search sanity checks ───────────────────────── */

    marlin()
        .arg("search").arg("TODO")
        .assert()
        .stdout(predicate::str::contains("TODO.txt"));

    marlin()
        .arg("search").arg("attr:reviewed=yes")
        .assert()
        .stdout(predicate::str::contains("Q1.pdf"));

    /* ── 5 ░ link flow & backlinks ────────────────────────────── */

    let foo = demo_dir.join("foo.txt");
    let bar = demo_dir.join("bar.txt");
    fs::write(&foo, "")?;
    fs::write(&bar, "")?;

    ok(marlin().arg("scan").arg(&demo_dir));

    ok(marlin()
        .arg("link").arg("add")
        .arg(&foo).arg(&bar));

    marlin()
        .arg("link").arg("backlinks").arg(&bar)
        .assert()
        .stdout(predicate::str::contains("foo.txt"));

    /* ── 6 ░ backup → delete DB → restore ────────────────────── */

    let backup_path = String::from_utf8(
        marlin().arg("backup").output()?.stdout
    )?;
    let backup_file = backup_path.split_whitespace().last().unwrap();

    fs::remove_file(&db_path)?;                        // simulate corruption
    ok(marlin().arg("restore").arg(backup_file));      // restore

    // Search must still work afterwards
    marlin()
        .arg("search").arg("TODO")
        .assert()
        .stdout(predicate::str::contains("TODO.txt"));

    Ok(())
}

