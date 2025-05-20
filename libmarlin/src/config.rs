use anyhow::Result;
use directories::ProjectDirs;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

/// Runtime configuration (currently just the DB path).
#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
}

impl Config {
    /// Resolve configuration from environment or derive one per-workspace.
    ///
    /// Priority:
    /// 1. `MARLIN_DB_PATH` env-var (explicit override)
    /// 2. *Workspace-local* file under XDG data dir
    ///    (`~/.local/share/marlin/index_<hash>.db`)
    /// 3. Fallback to   `./index.db`  when we cannot locate an XDG dir
    pub fn load() -> Result<Self> {
        // 1) explicit override
        if let Some(val) = std::env::var_os("MARLIN_DB_PATH") {
            let p = PathBuf::from(val);
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            return Ok(Self { db_path: p });
        }

        // 2) derive per-workspace DB name from CWD hash
        let cwd = std::env::current_dir()?;
        let mut h = DefaultHasher::new();
        cwd.hash(&mut h);
        let digest = h.finish(); // 64-bit
        let file_name = format!("index_{digest:016x}.db");

        if let Some(dirs) = ProjectDirs::from("io", "Marlin", "marlin") {
            let dir = dirs.data_dir();
            std::fs::create_dir_all(dir)?;
            return Ok(Self {
                db_path: dir.join(file_name),
            });
        }

        // 3) very last resort – workspace-relative DB
        Ok(Self {
            db_path: Path::new(&file_name).to_path_buf(),
        })
    }
}
