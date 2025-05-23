// libmarlin/src/backup.rs

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use rusqlite;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::warn;

use crate::error as marlin_error;

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

#[derive(Debug)]
pub struct BackupManager {
    live_db_path: PathBuf,
    backups_dir: PathBuf,
}

impl BackupManager {
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(
        live_db_path: P1,
        backups_dir: P2,
    ) -> Result<Self> {
        let backups_dir_path = backups_dir.as_ref().to_path_buf();
        if !backups_dir_path.exists() {
            fs::create_dir_all(&backups_dir_path).with_context(|| {
                format!(
                    "Failed to create backup directory at {}",
                    backups_dir_path.display()
                )
            })?;
        } else if !backups_dir_path.is_dir() {
            return Err(anyhow!(
                "Backups path exists but is not a directory: {}",
                backups_dir_path.display()
            ));
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

        if !self.live_db_path.exists() {
            return Err(anyhow::Error::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Live DB path does not exist: {}",
                    self.live_db_path.display()
                ),
            ))
            .context("Cannot create backup from non-existent live DB"));
        }

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

        backup_op
            .run_to_completion(100, Duration::from_millis(250), None)
            .map_err(|e| anyhow::Error::new(e).context("SQLite backup operation failed"))?;

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

        if !self.backups_dir.exists() {
            return Ok(backup_infos);
        }

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
                            let metadata = fs::metadata(&path).with_context(|| {
                                format!("Failed to get metadata for {}", path.display())
                            })?;

                            let ts_str = filename
                                .trim_start_matches("backup_")
                                .trim_end_matches(".db");

                            let parsed_dt =
                                NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d_%H-%M-%S_%f")
                                    .or_else(|_| {
                                        NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d_%H-%M-%S")
                                    });

                            let timestamp_utc = match parsed_dt {
                                Ok(naive_dt) => {
                                    let local_dt_result = Local.from_local_datetime(&naive_dt);
                                    let local_dt = match local_dt_result {
                                        chrono::LocalResult::Single(dt) => dt,
                                        chrono::LocalResult::Ambiguous(dt1, _dt2) => {
                                            warn!(
                                                "Ambiguous local time for backup {}, taking first interpretation",
                                                filename
                                            );
                                            dt1
                                        }
                                        chrono::LocalResult::None => {
                                            warn!(
                                                "Invalid local time for backup {}, skipping",
                                                filename
                                            );
                                            continue;
                                        }
                                    };
                                    DateTime::<Utc>::from(local_dt)
                                }
                                Err(_) => DateTime::<Utc>::from(metadata.modified()?),
                            };

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

        if keep_count >= all_backups.len() {
            kept = all_backups;
        } else {
            for (index, backup_info) in all_backups.into_iter().enumerate() {
                if index < keep_count {
                    kept.push(backup_info);
                } else {
                    let backup_file_path = self.backups_dir.join(&backup_info.id);
                    if backup_file_path.exists() {
                        fs::remove_file(&backup_file_path).with_context(|| {
                            format!(
                                "Failed to remove old backup file: {}",
                                backup_file_path.display()
                            )
                        })?;
                    }
                    removed.push(backup_info);
                }
            }
        }
        Ok(PruneResult { kept, removed })
    }

    pub fn verify_backup(&self, backup_id: &str) -> Result<bool> {
        let backup_file_path = self.backups_dir.join(backup_id);
        if !backup_file_path.exists() || !backup_file_path.is_file() {
            return Err(anyhow::Error::new(marlin_error::Error::NotFound(format!(
                "Backup file not found or is not a file: {}",
                backup_file_path.display()
            ))));
        }
        let conn = rusqlite::Connection::open(&backup_file_path)?;
        let res: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        Ok(res == "ok")
    }

    pub fn restore_from_backup(&self, backup_id: &str) -> Result<()> {
        let backup_file_path = self.backups_dir.join(backup_id);
        if !backup_file_path.exists() || !backup_file_path.is_file() {
            return Err(anyhow::Error::new(marlin_error::Error::NotFound(format!(
                "Backup file not found or is not a file: {}",
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
    use crate::db::open as open_marlin_db;
    use std::sync::Once;
    use tempfile::tempdir;

    static INIT: Once = Once::new();

    fn init_logging() {
        INIT.call_once(|| {
            crate::logging::init();
        });
    }

    fn create_valid_live_db(path: &Path) -> rusqlite::Connection {
        let conn = open_marlin_db(path).unwrap_or_else(|e| {
            panic!(
                "Failed to open/create test DB at {}: {:?}",
                path.display(),
                e
            )
        });
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS test_table (id INTEGER PRIMARY KEY, data TEXT);
             INSERT INTO test_table (data) VALUES ('initial_data');",
        )
        .expect("Failed to initialize test table");
        conn
    }

    #[test]
    fn test_backup_manager_new_creates_dir() {
        init_logging();
        let base_tmp = tempdir().unwrap();
        let live_db_path = base_tmp.path().join("live_new_creates.db");
        let _conn = create_valid_live_db(&live_db_path);

        let backups_dir = base_tmp.path().join("my_backups_new_creates_test");

        assert!(!backups_dir.exists());
        let manager = BackupManager::new(&live_db_path, &backups_dir).unwrap();
        assert!(manager.backups_dir.exists());
        assert!(backups_dir.exists());
    }

    #[test]
    fn test_backup_manager_new_with_existing_dir() {
        init_logging();
        let base_tmp = tempdir().unwrap();
        let live_db_path = base_tmp.path().join("live_existing_dir.db");
        let _conn = create_valid_live_db(&live_db_path);

        let backups_dir = base_tmp.path().join("my_backups_existing_test");
        std::fs::create_dir_all(&backups_dir).unwrap();

        assert!(backups_dir.exists());
        let manager_res = BackupManager::new(&live_db_path, &backups_dir);
        assert!(manager_res.is_ok());
        let manager = manager_res.unwrap();
        assert_eq!(manager.backups_dir, backups_dir);
    }

    #[test]
    fn test_backup_manager_new_fails_if_backup_path_is_file() {
        init_logging();
        let base_tmp = tempdir().unwrap();
        let live_db_path = base_tmp.path().join("live_backup_path_is_file.db");
        let _conn = create_valid_live_db(&live_db_path);
        let file_as_backups_dir = base_tmp.path().join("file_as_backups_dir");
        std::fs::write(&file_as_backups_dir, "i am a file").unwrap();

        let manager_res = BackupManager::new(&live_db_path, &file_as_backups_dir);
        assert!(manager_res.is_err());
        assert!(manager_res
            .unwrap_err()
            .to_string()
            .contains("Backups path exists but is not a directory"));
    }

    #[test]
    fn test_create_backup_failure_non_existent_live_db() {
        init_logging();
        let base_tmp = tempdir().unwrap();
        let live_db_path = base_tmp.path().join("non_existent_live.db");
        let backups_dir = base_tmp.path().join("backups_fail_test");

        let manager = BackupManager::new(&live_db_path, &backups_dir).unwrap();
        let backup_result = manager.create_backup();
        assert!(backup_result.is_err());
        let err_str = backup_result.unwrap_err().to_string();
        assert!(
            err_str.contains("Cannot create backup from non-existent live DB")
                || err_str.contains("Failed to open source DB")
        );
    }

    #[test]
    fn test_create_list_prune_backups() {
        init_logging();
        let tmp = tempdir().unwrap();
        let live_db_file = tmp.path().join("live_for_clp_test.db");
        let _conn_live = create_valid_live_db(&live_db_file);

        let backups_storage_dir = tmp.path().join("backups_clp_storage_test");

        let manager = BackupManager::new(&live_db_file, &backups_storage_dir).unwrap();

        let initial_list = manager.list_backups().unwrap();
        assert!(
            initial_list.is_empty(),
            "Backup list should be empty initially"
        );

        let prune_empty_result = manager.prune(2).unwrap();
        assert!(prune_empty_result.kept.is_empty());
        assert!(prune_empty_result.removed.is_empty());

        let mut created_backup_ids = Vec::new();
        for i in 0..5 {
            let info = manager
                .create_backup()
                .unwrap_or_else(|e| panic!("Failed to create backup {}: {:?}", i, e));
            created_backup_ids.push(info.id.clone());
            std::thread::sleep(std::time::Duration::from_millis(30));
        }

        let listed_backups = manager.list_backups().unwrap();
        assert_eq!(listed_backups.len(), 5);
        for id in &created_backup_ids {
            assert!(
                listed_backups.iter().any(|b| &b.id == id),
                "Backup ID {} not found in list",
                id
            );
        }
        if listed_backups.len() >= 2 {
            assert!(listed_backups[0].timestamp >= listed_backups[1].timestamp);
        }

        let prune_to_zero_result = manager.prune(0).unwrap();
        assert_eq!(prune_to_zero_result.kept.len(), 0);
        assert_eq!(prune_to_zero_result.removed.len(), 5);
        let listed_after_prune_zero = manager.list_backups().unwrap();
        assert!(listed_after_prune_zero.is_empty());

        created_backup_ids.clear();
        for i in 0..5 {
            let info = manager
                .create_backup()
                .unwrap_or_else(|e| panic!("Failed to create backup {}: {:?}", i, e));
            created_backup_ids.push(info.id.clone());
            std::thread::sleep(std::time::Duration::from_millis(30));
        }

        let prune_keep_more_result = manager.prune(10).unwrap();
        assert_eq!(prune_keep_more_result.kept.len(), 5);
        assert_eq!(prune_keep_more_result.removed.len(), 0);
        let listed_after_prune_more = manager.list_backups().unwrap();
        assert_eq!(listed_after_prune_more.len(), 5);

        let prune_result = manager.prune(2).unwrap();
        assert_eq!(prune_result.kept.len(), 2);
        assert_eq!(prune_result.removed.len(), 3);

        let listed_after_prune = manager.list_backups().unwrap();
        assert_eq!(listed_after_prune.len(), 2);

        assert_eq!(listed_after_prune[0].id, created_backup_ids[4]);
        assert_eq!(listed_after_prune[1].id, created_backup_ids[3]);

        for removed_info in prune_result.removed {
            assert!(
                !backups_storage_dir.join(&removed_info.id).exists(),
                "Removed backup file {} should not exist",
                removed_info.id
            );
        }
        for kept_info in prune_result.kept {
            assert!(
                backups_storage_dir.join(&kept_info.id).exists(),
                "Kept backup file {} should exist",
                kept_info.id
            );
        }
    }

    #[test]
    fn test_restore_backup() {
        init_logging();
        let tmp = tempdir().unwrap();
        let live_db_path = tmp.path().join("live_for_restore_test.db");

        let initial_value = "initial_data_for_restore";
        {
            let conn = create_valid_live_db(&live_db_path);
            conn.execute("DELETE FROM test_table", []).unwrap();
            conn.execute("INSERT INTO test_table (data) VALUES (?1)", [initial_value])
                .unwrap();
        }

        let backups_dir = tmp.path().join("backups_for_restore_test_dir");
        let manager = BackupManager::new(&live_db_path, &backups_dir).unwrap();

        let backup_info = manager.create_backup().unwrap();

        let modified_value = "modified_data_for_restore";
        {
            let conn = rusqlite::Connection::open(&live_db_path)
                .expect("Failed to open live DB for modification");
            conn.execute("UPDATE test_table SET data = ?1", [modified_value])
                .expect("Failed to update data");
            let modified_check: String = conn
                .query_row("SELECT data FROM test_table", [], |row| row.get(0))
                .unwrap();
            assert_eq!(modified_check, modified_value);
        }

        manager.restore_from_backup(&backup_info.id).unwrap();

        {
            let conn_after_restore = rusqlite::Connection::open(&live_db_path)
                .expect("Failed to open live DB after restore");
            let restored_data: String = conn_after_restore
                .query_row("SELECT data FROM test_table", [], |row| row.get(0))
                .unwrap();
            assert_eq!(restored_data, initial_value);
        }
    }

    #[test]
    fn test_restore_non_existent_backup() {
        init_logging();
        let tmp = tempdir().unwrap();
        let live_db_path = tmp.path().join("live_for_restore_fail_test.db");
        let _conn = create_valid_live_db(&live_db_path);

        let backups_dir = tmp.path().join("backups_for_restore_fail_test");
        let manager = BackupManager::new(&live_db_path, &backups_dir).unwrap();

        let result = manager.restore_from_backup("non_existent_backup.db");
        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("Backup file not found"),
            "Error string was: {}",
            err_string
        );
    }

    #[test]
    fn list_backups_with_non_backup_files() {
        init_logging();
        let tmp = tempdir().unwrap();
        let live_db_file = tmp.path().join("live_for_list_test.db");
        let _conn = create_valid_live_db(&live_db_file);
        let backups_dir = tmp.path().join("backups_list_mixed_files_test");

        let manager = BackupManager::new(&live_db_file, &backups_dir).unwrap();

        manager.create_backup().unwrap();

        std::fs::write(backups_dir.join("not_a_backup.txt"), "hello").unwrap();
        std::fs::write(backups_dir.join("backup_malformed.db.tmp"), "temp data").unwrap();
        std::fs::create_dir(backups_dir.join("a_subdir")).unwrap();

        let listed_backups = manager.list_backups().unwrap();
        assert_eq!(
            listed_backups.len(),
            1,
            "Should only list the valid backup file"
        );
        assert!(listed_backups[0].id.starts_with("backup_"));
        assert!(listed_backups[0].id.ends_with(".db"));
    }

    #[test]
    fn list_backups_handles_io_error_on_read_dir() {
        init_logging();
        let tmp = tempdir().unwrap();
        let live_db_file = tmp.path().join("live_for_list_io_error.db");
        let _conn = create_valid_live_db(&live_db_file);

        let backups_dir_for_deletion = tmp.path().join("backups_dir_to_delete_test");
        let manager_for_deletion =
            BackupManager::new(&live_db_file, &backups_dir_for_deletion).unwrap();
        std::fs::remove_dir_all(&backups_dir_for_deletion).unwrap();

        let list_res = manager_for_deletion.list_backups().unwrap();
        assert!(list_res.is_empty());
    }

    #[test]
    fn list_backups_fallback_modification_time() {
        init_logging();
        let tmp = tempdir().unwrap();
        let live_db = tmp.path().join("live_for_badformat.db");
        let _conn = create_valid_live_db(&live_db);

        let backups_dir = tmp.path().join("backups_badformat_test");
        let manager = BackupManager::new(&live_db, &backups_dir).unwrap();

        let bad_backup_path = backups_dir.join("backup_badformat.db");
        std::fs::write(&bad_backup_path, b"bad").unwrap();

        let metadata = std::fs::metadata(&bad_backup_path).unwrap();
        let expected_ts = chrono::DateTime::<Utc>::from(metadata.modified().unwrap());

        let listed = manager.list_backups().unwrap();
        assert_eq!(listed.len(), 1);

        let info = &listed[0];
        assert_eq!(info.id, "backup_badformat.db");
        assert_eq!(info.timestamp, expected_ts);
    }

    #[test]
    fn verify_backup_ok() {
        init_logging();
        let tmp = tempdir().unwrap();
        let live_db = tmp.path().join("live_verify.db");
        let _conn = create_valid_live_db(&live_db);

        let backups_dir = tmp.path().join("ver_backups");
        let manager = BackupManager::new(&live_db, &backups_dir).unwrap();
        let info = manager.create_backup().unwrap();

        let ok = manager.verify_backup(&info.id).unwrap();
        assert!(ok, "expected integrity check to pass");
    }
}
