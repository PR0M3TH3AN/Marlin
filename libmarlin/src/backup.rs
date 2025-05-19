// libmarlin/src/backup.rs

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, Utc, TimeZone};
use rusqlite;
use std::fs; // This fs is for the BackupManager impl
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error as marlin_error;

// ... (BackupInfo, PruneResult, BackupManager struct and impl remain the same as previously corrected) ...
// (Ensure the BackupManager implementation itself is correct based on the previous fixes)
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: u64,
    pub hash: Option<String>,
}

#[derive(Debug)]
pub struct PruneResult {
    pub kept: Vec<BackupInfo>,
    pub removed: Vec<BackupInfo>,
}

pub struct BackupManager {
    live_db_path: PathBuf,
    backups_dir: PathBuf,
}

impl BackupManager {
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(live_db_path: P1, backups_dir: P2) -> Result<Self> {
        let backups_dir_path = backups_dir.as_ref().to_path_buf();
        if !backups_dir_path.exists() {
            fs::create_dir_all(&backups_dir_path).with_context(|| {
                format!(
                    "Failed to create backup directory at {}",
                    backups_dir_path.display()
                )
            })?;
        }
        Ok(Self {
            live_db_path: live_db_path.as_ref().to_path_buf(),
            backups_dir: backups_dir_path,
        })
    }

    pub fn create_backup(&self) -> Result<BackupInfo> {
        let stamp = Local::now().format("%Y-%m-%d_%H-%M-%S_%f");
        let backup_file_name = format!("backup_{stamp}.db");
        let backup_file_path = self.backups_dir.join(&backup_file_name);

        let src_conn = rusqlite::Connection::open_with_flags(
            &self.live_db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .with_context(|| {
            format!(
                "Failed to open source DB ('{}') for backup",
                self.live_db_path.display()
            )
        })?;

        let mut dst_conn = rusqlite::Connection::open(&backup_file_path).with_context(|| {
            format!(
                "Failed to open destination backup file: {}",
                backup_file_path.display()
            )
        })?;

        let backup_op =
            rusqlite::backup::Backup::new(&src_conn, &mut dst_conn).with_context(|| {
                format!(
                    "Failed to initialize backup from {} to {}",
                    self.live_db_path.display(),
                    backup_file_path.display()
                )
            })?;

        match backup_op.run_to_completion(100, Duration::from_millis(250), None) {
            Ok(_) => (),
            Err(e) => return Err(anyhow::Error::new(e).context("SQLite backup operation failed")),
        };

        let metadata = fs::metadata(&backup_file_path).with_context(|| {
            format!(
                "Failed to get metadata for backup file: {}",
                backup_file_path.display()
            )
        })?;

        Ok(BackupInfo {
            id: backup_file_name,
            timestamp: DateTime::from(metadata.modified()?),
            size_bytes: metadata.len(),
            hash: None,
        })
    }

    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let mut backup_infos = Vec::new();

        for entry_result in fs::read_dir(&self.backups_dir).with_context(|| {
            format!(
                "Failed to read backup directory: {}",
                self.backups_dir.display()
            )
        })? {
            let entry = entry_result?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename_osstr) = path.file_name() {
                    if let Some(filename) = filename_osstr.to_str() {
                        if filename.starts_with("backup_") && filename.ends_with(".db") {
                            let ts_str = filename
                                .trim_start_matches("backup_")
                                .trim_end_matches(".db");
                            
                            let naive_dt = match NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d_%H-%M-%S_%f") {
                                Ok(dt) => dt,
                                Err(_) => match NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d_%H-%M-%S") {
                                    Ok(dt) => dt,
                                    Err(_) => {
                                        let metadata = fs::metadata(&path)?;
                                        DateTime::<Utc>::from(metadata.modified()?).naive_utc()
                                    }
                                }
                            };
                            
                            let local_dt_result = Local.from_local_datetime(&naive_dt);
                            let local_dt = match local_dt_result {
                                chrono::LocalResult::Single(dt) => dt,
                                chrono::LocalResult::Ambiguous(dt1, _dt2) => {
                                    eprintln!("Warning: Ambiguous local time for backup {}, taking first interpretation.", filename);
                                    dt1
                                },
                                chrono::LocalResult::None => {
                                    return Err(anyhow!("Invalid local time for backup {}", filename));
                                }
                            };
                            let timestamp_utc = DateTime::<Utc>::from(local_dt);

                            let metadata = fs::metadata(&path)?;
                            backup_infos.push(BackupInfo {
                                id: filename.to_string(),
                                timestamp: timestamp_utc,
                                size_bytes: metadata.len(),
                                hash: None,
                            });
                        }
                    }
                }
            }
        }
        backup_infos.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(backup_infos)
    }

    pub fn prune(&self, keep_count: usize) -> Result<PruneResult> {
        let all_backups = self.list_backups()?; 

        let mut kept = Vec::new();
        let mut removed = Vec::new();

        for (index, backup_info) in all_backups.into_iter().enumerate() {
            if index < keep_count {
                kept.push(backup_info);
            } else {
                let backup_file_path = self.backups_dir.join(&backup_info.id);
                fs::remove_file(&backup_file_path).with_context(|| {
                    format!(
                        "Failed to remove old backup file: {}",
                        backup_file_path.display()
                    )
                })?;
                removed.push(backup_info);
            }
        }
        Ok(PruneResult { kept, removed })
    }

    pub fn restore_from_backup(&self, backup_id: &str) -> Result<()> {
        let backup_file_path = self.backups_dir.join(backup_id);
        if !backup_file_path.exists() {
            return Err(anyhow::Error::new(marlin_error::Error::NotFound(format!(
                "Backup file not found: {}",
                backup_file_path.display()
            ))));
        }

        fs::copy(&backup_file_path, &self.live_db_path).with_context(|| {
            format!(
                "Failed to copy backup {} to live DB {}",
                backup_file_path.display(),
                self.live_db_path.display()
            )
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    // use std::fs; // <-- REMOVE this line if not directly used by tests
    use crate::db::open as open_marlin_db;

    #[test]
    fn test_backup_manager_new_creates_dir() {
        let base_tmp = tempdir().unwrap();
        let live_db_path = base_tmp.path().join("live.db");
        
        let _conn = open_marlin_db(&live_db_path).expect("Failed to open test live DB for new_creates_dir test");

        let backups_dir = base_tmp.path().join("my_backups_new_creates");

        assert!(!backups_dir.exists());
        let manager = BackupManager::new(&live_db_path, &backups_dir).unwrap();
        assert!(manager.backups_dir.exists()); 
        assert!(backups_dir.exists());
    }

    #[test]
    fn test_create_list_prune_backups() {
        let tmp = tempdir().unwrap();
        let live_db_file = tmp.path().join("live_for_clp.db");

        let _conn_live = open_marlin_db(&live_db_file).expect("Failed to open live_db_file for clp test");

        let backups_storage_dir = tmp.path().join("backups_clp_storage");
        
        let manager = BackupManager::new(&live_db_file, &backups_storage_dir).unwrap();

        let mut created_backup_ids = Vec::new();
        for i in 0..5 {
            let info = manager.create_backup().unwrap_or_else(|e| panic!("Failed to create backup {}: {:?}", i, e) );
            created_backup_ids.push(info.id.clone()); 
            std::thread::sleep(std::time::Duration::from_millis(30));
        }

        let listed_backups = manager.list_backups().unwrap();
        assert_eq!(listed_backups.len(), 5);
        for id in &created_backup_ids {
            assert!(listed_backups.iter().any(|b| &b.id == id), "Backup ID {} not found in list", id);
        }

        let prune_result = manager.prune(2).unwrap();
        assert_eq!(prune_result.kept.len(), 2);
        assert_eq!(prune_result.removed.len(), 3);

        let listed_after_prune = manager.list_backups().unwrap();
        assert_eq!(listed_after_prune.len(), 2);

        assert_eq!(listed_after_prune[0].id, created_backup_ids[4]);
        assert_eq!(listed_after_prune[1].id, created_backup_ids[3]);
        
        for removed_info in prune_result.removed {
            assert!(!backups_storage_dir.join(&removed_info.id).exists(), "Removed backup file {} should not exist", removed_info.id);
        }
        for kept_info in prune_result.kept {
            assert!(backups_storage_dir.join(&kept_info.id).exists(), "Kept backup file {} should exist", kept_info.id);
        }
    }

     #[test]
    fn test_restore_backup() {
        let tmp = tempdir().unwrap();
        let live_db_path = tmp.path().join("live_for_restore.db");
        
        let initial_value = "initial_data_for_restore";
        {
            // FIX 2: Remove `mut`
            let conn = open_marlin_db(&live_db_path).expect("Failed to open initial live_db_path for restore test");
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS verify_restore (id INTEGER PRIMARY KEY, data TEXT);"
            ).expect("Failed to create verify_restore table");
            conn.execute("INSERT INTO verify_restore (data) VALUES (?1)", [initial_value]).expect("Failed to insert initial data");
        }

        let backups_dir = tmp.path().join("backups_for_restore_test");
        let manager = BackupManager::new(&live_db_path, &backups_dir).unwrap();

        let backup_info = manager.create_backup().unwrap();

        let modified_value = "modified_data_for_restore";
        {
            // FIX 3: Remove `mut`
            let conn = rusqlite::Connection::open(&live_db_path).expect("Failed to open live DB for modification");
            conn.execute("UPDATE verify_restore SET data = ?1", [modified_value]).expect("Failed to update data");
            let modified_check: String = conn.query_row("SELECT data FROM verify_restore", [], |row| row.get(0)).unwrap();
            assert_eq!(modified_check, modified_value);
        }
        
        manager.restore_from_backup(&backup_info.id).unwrap();

        {
            let conn_after_restore = rusqlite::Connection::open(&live_db_path).expect("Failed to open live DB after restore");
            let restored_data: String = conn_after_restore.query_row("SELECT data FROM verify_restore", [], |row| row.get(0)).unwrap();
            assert_eq!(restored_data, initial_value);
        }
    }
}