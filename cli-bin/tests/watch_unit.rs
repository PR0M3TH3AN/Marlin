use std::thread;
use std::time::Duration;
use tempfile::tempdir;

use libmarlin::{self as marlin, db};
use marlin_cli::cli::watch::WatchCmd;
use marlin_cli::cli::{watch, Format};

#[cfg(unix)]
#[test]
fn watch_start_and_stop_quickly() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("index.db");
    std::env::set_var("MARLIN_DB_PATH", &db_path);

    let _m = marlin::Marlin::open_default().unwrap();

    let mut conn = db::open(&db_path).unwrap();

    let path = tmp.path().to_path_buf();
    let cmd = WatchCmd::Start {
        path: path.clone(),
        debounce_ms: 50,
    };

    watch::run(&cmd, &mut conn, Format::Text).unwrap();
    thread::sleep(Duration::from_millis(500));

    watch::run(&WatchCmd::Status, &mut conn, Format::Text).unwrap();
    watch::run(&WatchCmd::Stop, &mut conn, Format::Text).unwrap();

    let cfg = libmarlin::config::Config::load().unwrap();
    let control = cfg.db_path.with_extension("watch.json");
    assert!(!control.exists());
}
