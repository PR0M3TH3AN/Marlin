//! tests neg.rs
//! Negative-path integration tests (“should fail / warn”).

use predicates::str;
use tempfile::tempdir;

mod util;
use util::marlin;

/* ───────────────────────── LINKS ─────────────────────────────── */

#[test]
fn link_non_indexed_should_fail() {
    let tmp = tempdir().unwrap();

    marlin(&tmp)
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .success();

    std::fs::write(tmp.path().join("foo.txt"), "").unwrap();
    std::fs::write(tmp.path().join("bar.txt"), "").unwrap();

    marlin(&tmp)
        .current_dir(tmp.path())
        .args([
            "link",
            "add",
            &tmp.path().join("foo.txt").to_string_lossy(),
            &tmp.path().join("bar.txt").to_string_lossy(),
        ])
        .assert()
        .failure()
        .stderr(str::contains("file not indexed"));
}

/* ───────────────────────── ATTR ─────────────────────────────── */

#[test]
fn attr_set_on_non_indexed_file_should_warn() {
    let tmp = tempdir().unwrap();
    marlin(&tmp)
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .success();

    let ghost = tmp.path().join("ghost.txt");
    std::fs::write(&ghost, "").unwrap();

    marlin(&tmp)
        .args(["attr", "set", &ghost.to_string_lossy(), "foo", "bar"])
        .assert()
        .success() // exits 0
        .stderr(str::contains("not indexed"));
}

/* ───────────────────── COLLECTIONS ───────────────────────────── */

#[test]
fn coll_add_unknown_collection_should_fail() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("doc.txt");
    std::fs::write(&file, "").unwrap();

    marlin(&tmp)
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .success();

    marlin(&tmp)
        .args(["coll", "add", "nope", &file.to_string_lossy()])
        .assert()
        .failure();
}

/* ───────────────────── RESTORE (bad file) ───────────────────── */

#[test]
fn restore_with_nonexistent_backup_should_fail() {
    let tmp = tempdir().unwrap();

    // create an empty DB first
    marlin(&tmp).arg("init").assert().success();

    marlin(&tmp)
        .args(["restore", "/definitely/not/here.db"])
        .assert()
        .failure()
        .stderr(str::contains("Failed to restore"));
}
