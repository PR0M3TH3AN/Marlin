//! Tests for the file system watcher functionality

#[cfg(test)]
mod tests {
    // Updated import for BackupManager from the new backup module
    use crate::backup::BackupManager;
    // These are still from the watcher module
    use crate::watcher::{FileWatcher, WatcherConfig, WatcherState};
    use crate::db::open as open_marlin_db; // Use your project's DB open function


    use std::fs::{self, File};
    use std::io::Write;
    // No longer need: use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_watcher_lifecycle() {
        // Create a temp directory for testing
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        // Create a test file
        let test_file_path = temp_path.join("test.txt");
        let mut file = File::create(&test_file_path).expect("Failed to create test file");
        writeln!(file, "Test content").expect("Failed to write to test file");
        drop(file);

        // Configure and start the watcher
        let config = WatcherConfig {
            debounce_ms: 100,
            batch_size: 10,
            max_queue_size: 100,
            drain_timeout_ms: 1000,
        };

        let mut watcher = FileWatcher::new(vec![temp_path.to_path_buf()], config)
            .expect("Failed to create watcher");

        watcher.start().expect("Failed to start watcher");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Watching);

        thread::sleep(Duration::from_millis(200));
        let new_file_path = temp_path.join("new_file.txt");
        let mut new_file_handle = File::create(&new_file_path).expect("Failed to create new file");
        writeln!(new_file_handle, "New file content").expect("Failed to write to new file");
        drop(new_file_handle);

        thread::sleep(Duration::from_millis(200));
        let mut existing_file_handle = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&test_file_path)
            .expect("Failed to open test file for modification");
        writeln!(existing_file_handle, "Additional content").expect("Failed to append to test file");
        drop(existing_file_handle);

        thread::sleep(Duration::from_millis(200));
        fs::remove_file(&new_file_path).expect("Failed to remove file");

        thread::sleep(Duration::from_millis(500));
        watcher.stop().expect("Failed to stop watcher");

        assert_eq!(watcher.status().unwrap().state, WatcherState::Stopped);
        assert!(watcher.status().unwrap().events_processed > 0, "Expected some file events to be processed");
    }

    #[test]
    fn test_backup_manager_related_functionality() {
        let live_db_tmp_dir = tempdir().expect("Failed to create temp directory for live DB");
        let backups_storage_tmp_dir = tempdir().expect("Failed to create temp directory for backups storage");
        
        let live_db_path = live_db_tmp_dir.path().join("test_live_watcher.db"); // Unique name
        let backups_actual_dir = backups_storage_tmp_dir.path().join("my_backups_watcher"); // Unique name

        // Initialize a proper SQLite DB for the "live" database
        let _conn = open_marlin_db(&live_db_path).expect("Failed to open test_live_watcher.db for backup test");
        
        let backup_manager = BackupManager::new(&live_db_path, &backups_actual_dir)
            .expect("Failed to create BackupManager instance");
        
        let backup_info = backup_manager.create_backup().expect("Failed to create first backup");
        
        assert!(backups_actual_dir.join(&backup_info.id).exists(), "Backup file should exist");
        assert!(backup_info.size_bytes > 0, "Backup size should be greater than 0");
        
        for i in 0..3 {
            std::thread::sleep(std::time::Duration::from_millis(30)); // Ensure timestamp difference
            backup_manager.create_backup().unwrap_or_else(|e| panic!("Failed to create additional backup {}: {:?}", i, e));
        }
        
        let backups = backup_manager.list_backups().expect("Failed to list backups");
        assert_eq!(backups.len(), 4, "Should have 4 backups listed");
        
        let prune_result = backup_manager.prune(2).expect("Failed to prune backups");
        
        assert_eq!(prune_result.kept.len(), 2, "Should have kept 2 backups");
        assert_eq!(prune_result.removed.len(), 2, "Should have removed 2 backups (4 initial - 2 kept)");
        
        let remaining_backups = backup_manager.list_backups().expect("Failed to list backups after prune");
        assert_eq!(remaining_backups.len(), 2, "Should have 2 backups remaining after prune");

        for removed_info in prune_result.removed {
            assert!(!backups_actual_dir.join(&removed_info.id).exists(), "Removed backup file {} should not exist", removed_info.id);
        }
        for kept_info in prune_result.kept {
            assert!(backups_actual_dir.join(&kept_info.id).exists(), "Kept backup file {} should exist", kept_info.id);
        }
    }
}
