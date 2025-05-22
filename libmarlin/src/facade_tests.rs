// libmarlin/src/facade_tests.rs

use super::*; // brings Marlin, config, etc.
use crate::test_utils::ENV_MUTEX;
use std::{env, fs};
use tempfile::tempdir;

#[test]
fn open_at_and_scan_and_search() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // 1) Prepare a temp workspace with one file
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("hello.txt");
    fs::write(&file, "hello FAÇT").unwrap();

    // 2) Use open_at to create a fresh DB
    let db_path = tmp.path().join("explicit.db");
    let mut m = Marlin::open_at(&db_path).expect("open_at should succeed");
    assert!(db_path.exists(), "DB file should be created");

    // 3) Scan the directory
    let count = m.scan(&[tmp.path()]).expect("scan should succeed");
    assert_eq!(count, 1, "we created exactly one file");

    // 4) Search using an FTS hit
    let hits = m.search("hello").expect("search must not error");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].ends_with("hello.txt"));

    // 5) Search a substring that isn't a valid token (fires fallback)
    let fallback_hits = m.search("FAÇT").expect("fallback search works");
    assert_eq!(fallback_hits.len(), 1);
    assert!(fallback_hits[0].ends_with("hello.txt"));
}

#[test]
fn tag_and_search_by_tag() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let tmp = tempdir().unwrap();
    let a = tmp.path().join("a.md");
    let b = tmp.path().join("b.md");
    fs::write(&a, "# a").unwrap();
    fs::write(&b, "# b").unwrap();

    let db_path = tmp.path().join("my.db");
    env::set_var("MARLIN_DB_PATH", &db_path);

    let mut m = Marlin::open_default().unwrap();
    m.scan(&[tmp.path()]).unwrap();

    let changed = m.tag("*.md", "foo/bar").unwrap();
    assert_eq!(changed, 2);

    let tagged = m.search("tags_text:\"foo/bar\"").unwrap();
    assert_eq!(tagged.len(), 2);

    env::remove_var("MARLIN_DB_PATH");
}

#[test]
fn open_default_fallback_config() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // Unset all overrides
    env::remove_var("MARLIN_DB_PATH");
    env::remove_var("XDG_DATA_HOME");

    // Simulate no XDG: temporarily point HOME to a read-only dir
    let fake_home = tempdir().unwrap();
    env::set_var("HOME", fake_home.path());
    // This should fall back to "./index_<hash>.db"
    let cfg = config::Config::load().unwrap();
    let fname = cfg.db_path.file_name().unwrap().to_string_lossy();
    assert!(fname.starts_with("index_") && fname.ends_with(".db"));

    // Clean up
    env::remove_var("HOME");
}
