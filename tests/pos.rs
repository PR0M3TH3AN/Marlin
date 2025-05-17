//! Positive-path integration checks for every sub-command
//! that already has real logic behind it.

mod util;
use util::marlin;

use predicates::{prelude::*, str};          // brings `PredicateBooleanExt::and`
use std::fs;
use tempfile::tempdir;

/* ─────────────────────────── TAG ─────────────────────────────── */

#[test]
fn tag_should_add_hierarchical_tag_and_search_finds_it() {
    let tmp  = tempdir().unwrap();
    let file = tmp.path().join("foo.md");
    fs::write(&file, "# test\n").unwrap();

    marlin(&tmp).current_dir(tmp.path()).arg("init").assert().success();

    marlin(&tmp)
        .args(["tag", file.to_str().unwrap(), "project/md"])
        .assert().success();

    marlin(&tmp)
        .args(["search", "tag:project/md"])
        .assert()
        .success()
        .stdout(str::contains("foo.md"));
}

/* ─────────────────────────── ATTR ────────────────────────────── */

#[test]
fn attr_set_then_ls_roundtrip() {
    let tmp  = tempdir().unwrap();
    let file = tmp.path().join("report.pdf");
    fs::write(&file, "%PDF-1.4\n").unwrap();

    marlin(&tmp).current_dir(tmp.path()).arg("init").assert().success();

    marlin(&tmp)
        .args(["attr", "set", file.to_str().unwrap(), "reviewed", "yes"])
        .assert().success();

    marlin(&tmp)
        .args(["attr", "ls", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(str::contains("reviewed = yes"));
}

/* ─────────────────────── COLLECTIONS ────────────────────────── */

#[test]
fn coll_create_add_and_list() {
    let tmp = tempdir().unwrap();

    let a = tmp.path().join("a.txt");
    let b = tmp.path().join("b.txt");
    fs::write(&a, "").unwrap();
    fs::write(&b, "").unwrap();

    marlin(&tmp).current_dir(tmp.path()).arg("init").assert().success();

    marlin(&tmp).args(["coll", "create", "Set"]).assert().success();
    for f in [&a, &b] {
        marlin(&tmp).args(["coll", "add", "Set", f.to_str().unwrap()]).assert().success();
    }

    marlin(&tmp)
        .args(["coll", "list", "Set"])
        .assert()
        .success()
        .stdout(str::contains("a.txt").and(str::contains("b.txt")));
}

/* ─────────────────────────── VIEWS ───────────────────────────── */

#[test]
fn view_save_list_and_exec() {
    let tmp  = tempdir().unwrap();

    let todo = tmp.path().join("TODO.txt");
    fs::write(&todo, "remember the milk\n").unwrap();

    marlin(&tmp).current_dir(tmp.path()).arg("init").assert().success();

    // save & list
    marlin(&tmp).args(["view", "save", "tasks", "milk"]).assert().success();
    marlin(&tmp)
        .args(["view", "list"])
        .assert()
        .success()
        .stdout(str::contains("tasks: milk"));

    // exec
    marlin(&tmp)
        .args(["view", "exec", "tasks"])
        .assert()
        .success()
        .stdout(str::contains("TODO.txt"));
}

/* ─────────────────────────── LINKS ───────────────────────────── */

#[test]
fn link_add_rm_and_list() {
    let tmp = tempdir().unwrap();

    let foo = tmp.path().join("foo.txt");
    let bar = tmp.path().join("bar.txt");
    fs::write(&foo, "").unwrap();
    fs::write(&bar, "").unwrap();

    // handy closure
    let mc = || marlin(&tmp);

    mc().current_dir(tmp.path()).arg("init").assert().success();
    mc().args(["scan", tmp.path().to_str().unwrap()]).assert().success();

    // add
    mc().args(["link", "add", foo.to_str().unwrap(), bar.to_str().unwrap()])
        .assert().success();

    // list (outgoing default)
    mc().args(["link", "list", foo.to_str().unwrap()])
        .assert().success()
        .stdout(str::contains("foo.txt").and(str::contains("bar.txt")));

    // remove
    mc().args(["link", "rm", foo.to_str().unwrap(), bar.to_str().unwrap()])
        .assert().success();

    // list now empty
    mc().args(["link", "list", foo.to_str().unwrap()])
        .assert().success()
        .stdout(str::is_empty());
}

/* ─────────────────────── SCAN (multi-path) ───────────────────── */

#[test]
fn scan_with_multiple_paths_indexes_all() {
    let tmp = tempdir().unwrap();

    let dir_a = tmp.path().join("A");
    let dir_b = tmp.path().join("B");
    std::fs::create_dir_all(&dir_a).unwrap();
    std::fs::create_dir_all(&dir_b).unwrap();
    let f1 = dir_a.join("one.txt");
    let f2 = dir_b.join("two.txt");
    fs::write(&f1, "").unwrap();
    fs::write(&f2, "").unwrap();

    marlin(&tmp).current_dir(tmp.path()).arg("init").assert().success();

    // multi-path scan
    marlin(&tmp)
        .args(["scan", dir_a.to_str().unwrap(), dir_b.to_str().unwrap()])
        .assert().success();

    // both files findable
    for term in ["one.txt", "two.txt"] {
        marlin(&tmp).args(["search", term])
            .assert()
            .success()
            .stdout(str::contains(term));
    }
}

