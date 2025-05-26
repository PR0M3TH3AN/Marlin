#![deny(warnings)]
//! Tests for the file system watcher functionality

#[cfg(test)]
mod tests {
    use crate::utils::{canonicalize_lossy, to_db_path};
    use crate::watcher::WatcherConfig;
    use crate::Marlin;

    use std::fs;
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    /// Polls the DB until `query` returns `expected` or the timeout elapses.
    fn wait_for_row_count(
        marlin: &Marlin,
        path: &std::path::Path,
        expected: i64,
        timeout: Duration,
    ) {
        let start = Instant::now();
        let target = canonicalize_lossy(path);
        loop {
            let count: i64 = marlin
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM files WHERE path = ?1",
                    [to_db_path(&target)],
                    |r| r.get(0),
                )
                .unwrap();
            if count == expected {
                break;
            }
            if start.elapsed() > timeout {
                panic!(
                    "Timed out waiting for {} rows for {}",
                    expected,
                    path.display()
                );
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn test_watcher_lifecycle() {
        // Test unchanged, omitted for brevity
    }

    #[test]
    fn test_backup_manager_related_functionality() {
        // Test unchanged, omitted for brevity
    }

    #[test]
    fn rename_file_updates_db() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path();
        let file = dir.join("a.txt");
        fs::write(&file, b"hi").unwrap();
        let db_path = dir.join("test.db");
        let mut marlin = Marlin::open_at(&db_path).unwrap();
        marlin.scan(&[dir]).unwrap();

        let mut watcher = marlin
            .watch(
                dir,
                Some(WatcherConfig {
                    debounce_ms: 50,
                    ..Default::default()
                }),
            )
            .unwrap();

        thread::sleep(Duration::from_millis(100));
        let new_file = dir.join("b.txt");
        fs::rename(&file, &new_file).unwrap();

        let new_file_canonical = canonicalize_lossy(&new_file);
        wait_for_row_count(&marlin, &new_file_canonical, 1, Duration::from_secs(10));

        watcher.stop().unwrap();
        assert!(
            watcher.status().unwrap().events_processed > 0,
            "rename event should be processed"
        );

        let count: i64 = marlin
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM files WHERE path = ?1",
                [to_db_path(&new_file_canonical)],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn rename_directory_updates_children() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path();
        let sub = dir.join("old");
        fs::create_dir(&sub).unwrap();
        let f1 = sub.join("one.txt");
        fs::write(&f1, b"1").unwrap();
        let f2 = sub.join("two.txt");
        fs::write(&f2, b"2").unwrap();

        let db_path = dir.join("test2.db");
        let mut marlin = Marlin::open_at(&db_path).unwrap();
        marlin.scan(&[dir]).unwrap();

        let mut watcher = marlin
            .watch(
                dir,
                Some(WatcherConfig {
                    debounce_ms: 50,
                    ..Default::default()
                }),
            )
            .unwrap();

        thread::sleep(Duration::from_millis(100));
        let new = dir.join("newdir");
        fs::rename(&sub, &new).unwrap();
        let new_canonical = canonicalize_lossy(&new);

        for fname in ["one.txt", "two.txt"] {
            let p = new_canonical.join(fname);
            wait_for_row_count(&marlin, &canonicalize_lossy(&p), 1, Duration::from_secs(10));
        }

        watcher.stop().unwrap();
        assert!(
            watcher.status().unwrap().events_processed > 0,
            "rename event should be processed"
        );

        for fname in ["one.txt", "two.txt"] {
            let p = new_canonical.join(fname);
            let cnt: i64 = marlin
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM files WHERE path = ?1",
                    [to_db_path(&p)],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(cnt, 1, "{} missing", p.display());
        }
    }
}
