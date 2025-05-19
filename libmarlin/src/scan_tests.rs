// libmarlin/src/scan_tests.rs

use super::scan::scan_directory;
use super::db;
use tempfile::tempdir;
use std::fs::File;

#[test]
fn scan_directory_counts_files() {
    let tmp = tempdir().unwrap();

    // create a couple of files
    File::create(tmp.path().join("a.txt")).unwrap();
    File::create(tmp.path().join("b.log")).unwrap();

    // open an in-memory DB (runs migrations)
    let mut conn = db::open(":memory:").unwrap();

    let count = scan_directory(&mut conn, tmp.path()).unwrap();
    assert_eq!(count, 2);

    // ensure the paths were inserted
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM files").unwrap();
    let total: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
    assert_eq!(total, 2);
}
