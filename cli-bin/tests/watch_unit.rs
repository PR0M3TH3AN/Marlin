use std::thread;
use std::time::Duration;
use tempfile::tempdir;

use marlin_cli::cli::{watch, Format};
use marlin_cli::cli::watch::WatchCmd;
use libmarlin::watcher::WatcherState;
use libmarlin::{self as marlin, db};
use libc;

#[test]
fn watch_start_and_stop_quickly() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("index.db");
    std::env::set_var("MARLIN_DB_PATH", &db_path);

    // create database
    let _m = marlin::Marlin::open_default().unwrap();

    let mut conn = db::open(&db_path).unwrap();

    let path = tmp.path().to_path_buf();
    let cmd = WatchCmd::Start { path: path.clone(), debounce_ms: 50 };

    // send SIGINT shortly after watcher starts
    let t = thread::spawn(|| {
        thread::sleep(Duration::from_millis(200));
        unsafe { libc::raise(libc::SIGINT) };
    });

    watch::run(&cmd, &mut conn, Format::Text).unwrap();
    t.join().unwrap();

    assert_eq!(watch::last_watcher_state(), Some(WatcherState::Stopped));
}
