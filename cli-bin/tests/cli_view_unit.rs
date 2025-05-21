mod cli {
    #[derive(Clone, Copy, Debug)]
    pub enum Format {
        Text,
        Json,
    }
}

#[path = "../src/cli/view.rs"]
mod view;

use libmarlin::db;

#[test]
fn view_run_save_and_exec() {
    let mut conn = db::open(":memory:").unwrap();
    conn.execute(
        "INSERT INTO files(path,size,mtime) VALUES ('TODO.txt',0,0)",
        [],
    )
    .unwrap();

    let save = view::ViewCmd::Save(view::ArgsSave {
        view_name: "tasks".into(),
        query: "TODO".into(),
    });
    view::run(&save, &mut conn, cli::Format::Text).unwrap();

    let stored: String = conn
        .query_row("SELECT query FROM views WHERE name='tasks'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(stored, "TODO");

    let list = view::ViewCmd::List;
    view::run(&list, &mut conn, cli::Format::Text).unwrap();

    let exec = view::ViewCmd::Exec(view::ArgsExec {
        view_name: "tasks".into(),
    });
    view::run(&exec, &mut conn, cli::Format::Text).unwrap();
}
