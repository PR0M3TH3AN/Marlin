// libmarlin/src/config_tests.rs

use super::config::Config;
use crate::test_utils::ENV_MUTEX;
use std::env;
use tempfile::tempdir;

#[test]
fn load_env_override() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("custom.db");
    env::set_var("MARLIN_DB_PATH", &db);
    let cfg = Config::load().unwrap();
    assert_eq!(cfg.db_path, db);
    env::remove_var("MARLIN_DB_PATH");
}

#[test]
fn load_xdg_or_fallback() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // since XDG_DATA_HOME will normally be present, just test it doesn't error
    let cfg = Config::load().unwrap();
    assert!(cfg.db_path.to_string_lossy().ends_with(".db"));
}

#[test]
fn load_fallback_current_dir() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // Save and clear HOME & XDG_DATA_HOME
    let orig_home = env::var_os("HOME");
    let orig_xdg = env::var_os("XDG_DATA_HOME");
    env::remove_var("HOME");
    env::remove_var("XDG_DATA_HOME");
    env::remove_var("MARLIN_DB_PATH");

    let cfg = Config::load().unwrap();

    // Compute expected file name based on current directory hash
    let cwd = env::current_dir().unwrap();
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    cwd.hash(&mut h);
    let digest = h.finish();
    let expected_name = format!("index_{:016x}.db", digest);

    assert_eq!(cfg.db_path, std::path::PathBuf::from(&expected_name));
    assert!(cfg
        .db_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("index_"));

    // Restore environment variables
    match orig_home {
        Some(val) => env::set_var("HOME", val),
        None => env::remove_var("HOME"),
    }
    match orig_xdg {
        Some(val) => env::set_var("XDG_DATA_HOME", val),
        None => env::remove_var("XDG_DATA_HOME"),
    }
}
