mod cli {
    #[derive(Clone, Copy, Debug)]
    pub enum Format {
        Text,
        Json,
    }
}

#[path = "../src/cli/coll.rs"]
mod coll;

use libmarlin::db;

#[test]
fn coll_run_creates_and_adds() {
    let mut conn = db::open(":memory:").unwrap();
    conn.execute(
        "INSERT INTO files(path,size,mtime) VALUES ('a.txt',0,0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO files(path,size,mtime) VALUES ('b.txt',0,0)",
        [],
    )
    .unwrap();

    let create = coll::CollCmd::Create(coll::CreateArgs { name: "Set".into() });
    coll::run(&create, &mut conn, cli::Format::Text).unwrap();

    let coll_id: i64 = conn
        .query_row("SELECT id FROM collections WHERE name='Set'", [], |r| {
            r.get(0)
        })
        .unwrap();

    let add = coll::CollCmd::Add(coll::AddArgs {
        name: "Set".into(),
        file_pattern: "*.txt".into(),
    });
    coll::run(&add, &mut conn, cli::Format::Text).unwrap();

    let cnt: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM collection_files WHERE collection_id=?1",
            [coll_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cnt, 2);

    let list = coll::CollCmd::List(coll::ListArgs { name: "Set".into() });
    coll::run(&list, &mut conn, cli::Format::Text).unwrap();
}
