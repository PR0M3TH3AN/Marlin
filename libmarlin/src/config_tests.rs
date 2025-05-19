// libmarlin/src/config_tests.rs

use super::config::Config;
use std::env;
use tempfile::tempdir;

#[test]
fn load_env_override() {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("custom.db");
    env::set_var("MARLIN_DB_PATH", &db);
    let cfg = Config::load().unwrap();
    assert_eq!(cfg.db_path, db);
    env::remove_var("MARLIN_DB_PATH");
}

#[test]
fn load_xdg_or_fallback() {
    // since XDG_DATA_HOME will normally be present, just test it doesn't error
    let cfg = Config::load().unwrap();
    assert!(cfg.db_path.to_string_lossy().ends_with(".db"));
}
