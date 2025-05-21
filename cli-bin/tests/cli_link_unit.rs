mod cli {
    #[derive(Clone, Copy, Debug)]
    pub enum Format {
        Text,
        Json,
    }
}

#[path = "../src/cli/link.rs"]
mod link;

use libmarlin::db;

#[test]
fn link_run_add_and_rm() {
    let mut conn = db::open(":memory:").unwrap();
    conn.execute(
        "INSERT INTO files(path,size,mtime) VALUES ('foo.txt',0,0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO files(path,size,mtime) VALUES ('bar.txt',0,0)",
        [],
    )
    .unwrap();

    let add = link::LinkCmd::Add(link::LinkArgs {
        from: "foo.txt".into(),
        to: "bar.txt".into(),
        r#type: None,
    });
    link::run(&add, &mut conn, cli::Format::Text).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let list = link::LinkCmd::List(link::ListArgs {
        pattern: "foo.txt".into(),
        direction: None,
        r#type: None,
    });
    link::run(&list, &mut conn, cli::Format::Text).unwrap();

    let rm = link::LinkCmd::Rm(link::LinkArgs {
        from: "foo.txt".into(),
        to: "bar.txt".into(),
        r#type: None,
    });
    link::run(&rm, &mut conn, cli::Format::Text).unwrap();
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining, 0);
}
