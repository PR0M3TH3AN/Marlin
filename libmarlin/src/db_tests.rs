// libmarlin/src/db_tests.rs

use super::db;
use crate::utils::to_db_path;
use rusqlite::Connection;
use tempfile::tempdir;

fn open_mem() -> Connection {
    // helper to open an in-memory DB with migrations applied
    db::open(":memory:").expect("open in-memory DB")
}

#[test]
fn ensure_tag_path_creates_hierarchy() {
    let conn = open_mem();
    // create foo/bar/baz
    let leaf = db::ensure_tag_path(&conn, "foo/bar/baz").unwrap();
    // foo should exist as a root tag
    let foo: i64 = conn
        .query_row(
            "SELECT id FROM tags WHERE name='foo' AND parent_id IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    // bar should be child of foo
    let bar: i64 = conn
        .query_row(
            "SELECT id FROM tags WHERE name='bar' AND parent_id = ?1",
            [foo],
            |r| r.get(0),
        )
        .unwrap();
    // baz should be child of bar, and its ID is what we got back
    let baz: i64 = conn
        .query_row(
            "SELECT id FROM tags WHERE name='baz' AND parent_id = ?1",
            [bar],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(leaf, baz);
}

#[test]
fn upsert_attr_inserts_and_updates() {
    let conn = open_mem();
    // insert a dummy file
    conn.execute(
        "INSERT INTO files(path, size, mtime) VALUES (?1, 0, 0)",
        ["a.txt"],
    )
    .unwrap();
    let fid: i64 = conn
        .query_row("SELECT id FROM files WHERE path='a.txt'", [], |r| r.get(0))
        .unwrap();

    // insert
    db::upsert_attr(&conn, fid, "k", "v").unwrap();
    let v1: String = conn
        .query_row(
            "SELECT value FROM attributes WHERE file_id=?1 AND key='k'",
            [fid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(v1, "v");

    // update
    db::upsert_attr(&conn, fid, "k", "v2").unwrap();
    let v2: String = conn
        .query_row(
            "SELECT value FROM attributes WHERE file_id=?1 AND key='k'",
            [fid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(v2, "v2");
}

#[test]
fn file_id_returns_id_and_errors_on_missing() {
    let conn = open_mem();

    // insert a single file
    conn.execute(
        "INSERT INTO files(path, size, mtime) VALUES (?1, 0, 0)",
        ["exist.txt"],
    )
    .unwrap();

    // fetch its id via raw SQL
    let fid: i64 = conn
        .query_row("SELECT id FROM files WHERE path='exist.txt'", [], |r| {
            r.get(0)
        })
        .unwrap();

    // db::file_id should return the same id for existing paths
    let looked_up = db::file_id(&conn, "exist.txt").unwrap();
    assert_eq!(looked_up, fid);

    // querying a missing path should yield an error
    assert!(db::file_id(&conn, "missing.txt").is_err());
}

#[test]
fn add_and_remove_links_and_backlinks() {
    let conn = open_mem();
    // create two files
    conn.execute(
        "INSERT INTO files(path, size, mtime) VALUES (?1, 0, 0)",
        ["one.txt"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO files(path, size, mtime) VALUES (?1, 0, 0)",
        ["two.txt"],
    )
    .unwrap();
    let src: i64 = conn
        .query_row("SELECT id FROM files WHERE path='one.txt'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let dst: i64 = conn
        .query_row("SELECT id FROM files WHERE path='two.txt'", [], |r| {
            r.get(0)
        })
        .unwrap();

    // add a link of type "ref"
    db::add_link(&conn, src, dst, Some("ref")).unwrap();
    let out = db::list_links(&conn, "one%", None, None).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].2.as_deref(), Some("ref"));

    // backlinks should mirror
    let back = db::find_backlinks(&conn, "two%").unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].1.as_deref(), Some("ref"));

    // remove it
    db::remove_link(&conn, src, dst, Some("ref")).unwrap();
    let empty = db::list_links(&conn, "one%", None, None).unwrap();
    assert!(empty.is_empty());
}

#[test]
fn collections_roundtrip() {
    let conn = open_mem();
    // create collection "C"
    let cid = db::ensure_collection(&conn, "C").unwrap();

    // add a file
    conn.execute(
        "INSERT INTO files(path, size, mtime) VALUES (?1, 0, 0)",
        ["f.txt"],
    )
    .unwrap();
    let fid: i64 = conn
        .query_row("SELECT id FROM files WHERE path='f.txt'", [], |r| r.get(0))
        .unwrap();

    db::add_file_to_collection(&conn, cid, fid).unwrap();
    let files = db::list_collection(&conn, "C").unwrap();
    assert_eq!(files, vec!["f.txt".to_string()]);
}

#[test]
fn views_save_and_query() {
    let conn = open_mem();
    db::save_view(&conn, "v1", "some_query").unwrap();
    let all = db::list_views(&conn).unwrap();
    assert_eq!(all, vec![("v1".to_string(), "some_query".to_string())]);

    let q = db::view_query(&conn, "v1").unwrap();
    assert_eq!(q, "some_query");
}

#[test]
fn backup_and_restore_cycle() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("data.db");
    let live = db::open(&db_path).unwrap();

    // insert a file
    live.execute(
        "INSERT INTO files(path, size, mtime) VALUES (?1, 0, 0)",
        ["x.bin"],
    )
    .unwrap();

    // backup
    let backup = db::backup(&db_path).unwrap();
    // remove original
    std::fs::remove_file(&db_path).unwrap();
    // restore
    db::restore(&backup, &db_path).unwrap();

    // reopen and check that x.bin survived
    let conn2 = db::open(&db_path).unwrap();
    let cnt: i64 = conn2
        .query_row("SELECT COUNT(*) FROM files WHERE path='x.bin'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(cnt, 1);
}

mod dirty_helpers {
    use super::{db, open_mem};

    #[test]
    fn mark_and_take_dirty_works() {
        let conn = open_mem();
        conn.execute(
            "INSERT INTO files(path, size, mtime) VALUES (?1, 0, 0)",
            ["dummy.txt"],
        )
        .unwrap();
        let fid: i64 = conn
            .query_row("SELECT id FROM files WHERE path='dummy.txt'", [], |r| {
                r.get(0)
            })
            .unwrap();

        db::mark_dirty(&conn, fid).unwrap();
        db::mark_dirty(&conn, fid).unwrap();

        let dirty = db::take_dirty(&conn).unwrap();
        assert_eq!(dirty, vec![fid]);

        let empty = db::take_dirty(&conn).unwrap();
        assert!(empty.is_empty());
    }
}

#[test]
fn tables_exist_and_fts_triggers() {
    use super::Marlin;
    use std::fs;

    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut marlin = Marlin::open_at(&db_path).unwrap();

    // the DB file should exist after opening
    assert!(db_path.exists());

    // confirm required tables
    for table in ["links", "collections", "collection_files", "views"] {
        let cnt: i64 = marlin
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1, "missing table {table}");
    }

    // create a file to index
    let file_dir = tmp.path().join("files");
    fs::create_dir(&file_dir).unwrap();
    let file_path = file_dir.join("sample.txt");
    fs::write(&file_path, "hello world").unwrap();

    // index via public helper
    marlin.scan(&[&file_dir]).unwrap();
    marlin.tag("*.txt", "foo/bar").unwrap();

    let fid = db::file_id(marlin.conn(), file_path.to_str().unwrap()).unwrap();
    db::upsert_attr(marlin.conn(), fid, "color", "blue").unwrap();

    // The FTS index is contentless, so columns return empty strings. Instead
    // verify that searching for our tag and attribute yields the file path.
    let hits_tag: Vec<String> = marlin
        .conn()
        .prepare("SELECT f.path FROM files_fts JOIN files f ON f.id = files_fts.rowid WHERE files_fts MATCH 'foo'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(hits_tag.contains(&to_db_path(&file_path)));

    let hits_attr: Vec<String> = marlin
        .conn()
        .prepare(r#"SELECT f.path FROM files_fts JOIN files f ON f.id = files_fts.rowid WHERE files_fts MATCH '"color=blue"'"#)
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(hits_attr.contains(&to_db_path(&file_path)));
}
