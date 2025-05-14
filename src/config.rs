use std::path::{Path, PathBuf};

use anyhow::Result;
use directories::ProjectDirs;

/// Runtime configuration (currently just the DB path).
#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
}

impl Config {
    /// Resolve configuration from environment or XDG directories.
    pub fn load() -> Result<Self> {
        let db_path = std::env::var_os("MARLIN_DB_PATH")
            .map(PathBuf::from)
            .or_else(|| {
                ProjectDirs::from("io", "Marlin", "marlin")
                    .map(|dirs| dirs.data_dir().join("index.db"))
            })
            .unwrap_or_else(|| Path::new("index.db").to_path_buf());

        std::fs::create_dir_all(
            db_path
                .parent()
                .expect("db_path should always have a parent directory"),
        )?;

        Ok(Self { db_path })
    }
}
