//! Integration test for the file watcher functionality
//! 
//! Tests various aspects of the file system watcher including:
//! - Basic event handling (create, modify, delete files)
//! - Debouncing of events
//! - Hierarchical event coalescing
//! - Graceful shutdown and event draining

use marlin::watcher::{FileWatcher, WatcherConfig, WatcherState};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

// Mock filesystem event simulator inspired by inotify-sim
struct MockEventSimulator {
    temp_dir: PathBuf,
    files_created: Vec<PathBuf>,
}

impl MockEventSimulator {
    fn new(temp_dir: PathBuf) -> Self {
        Self {
            temp_dir,
            files_created: Vec::new(),
        }
    }

    fn create_file(&mut self, relative_path: &str, content: &str) -> PathBuf {
        let path = self.temp_dir.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        
        let mut file = File::create(&path).expect("Failed to create file");
        file.write_all(content.as_bytes()).expect("Failed to write content");
        
        self.files_created.push(path.clone());
        path
    }
    
    fn modify_file(&self, relative_path: &str, new_content: &str) -> PathBuf {
        let path = self.temp_dir.join(relative_path);
        let mut file = File::create(&path).expect("Failed to update file");
        file.write_all(new_content.as_bytes()).expect("Failed to write content");
        path
    }
    
    fn delete_file(&mut self, relative_path: &str) {
        let path = self.temp_dir.join(relative_path);
        fs::remove_file(&path).expect("Failed to delete file");
        
        self.files_created.retain(|p| p != &path);
    }
    
    fn create_burst(&mut self, count: usize, prefix: &str) -> Vec<PathBuf> {
        let mut paths = Vec::with_capacity(count);
        
        for i in 0..count {
            let file_path = format!("{}/burst_file_{}.txt", prefix, i);
            let path = self.create_file(&file_path, &format!("Content {}", i));
            paths.push(path);
            
            // Small delay to simulate rapid but not instantaneous file creation
            thread::sleep(Duration::from_micros(10));
        }
        
        paths
    }
    
    fn cleanup(&self) {
        // No need to do anything as tempdir will clean itself
    }
}

#[test]
fn test_basic_watch_functionality() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_path_buf();
    
    let mut simulator = MockEventSimulator::new(temp_path.clone());
    
    // Create a test file before starting the watcher
    let initial_file = simulator.create_file("initial.txt", "Initial content");
    
    // Configure and start the watcher
    let config = WatcherConfig {
        debounce_ms: 100,
        batch_size: 100,
        max_queue_size: 1000,
        drain_timeout_ms: 1000,
    };
    
    let mut watcher = FileWatcher::new(vec![temp_path.clone()], config)
        .expect("Failed to create file watcher");
    
    // Start the watcher in a separate thread
    let watcher_thread = thread::spawn(move || {
        watcher.start().expect("Failed to start watcher");
        
        // Let it run for a short time
        thread::sleep(Duration::from_secs(5));
        
        // Stop the watcher
        watcher.stop().expect("Failed to stop watcher");
        
        // Return the watcher for inspection
        watcher
    });
    
    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(500));
    
    // Generate events
    let file1 = simulator.create_file("test1.txt", "Hello, world!");
    thread::sleep(Duration::from_millis(200));
    
    let file2 = simulator.create_file("dir1/test2.txt", "Hello from subdirectory!");
    thread::sleep(Duration::from_millis(200));
    
    simulator.modify_file("test1.txt", "Updated content");
    thread::sleep(Duration::from_millis(200));
    
    simulator.delete_file("test1.txt");
    
    // Wait for watcher thread to complete
    let finished_watcher = watcher_thread.join().expect("Watcher thread panicked");
    
    // Check status after processing events
    let status = finished_watcher.status().unwrap();
    
    // Assertions
    assert_eq!(status.state, WatcherState::Stopped);
    assert!(status.events_processed > 0, "Expected events to be processed");
    assert_eq!(status.queue_size, 0, "Expected empty queue after stopping");
    
    // Clean up
    simulator.cleanup();
}

#[test]
fn test_debouncing() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_path_buf();
    
    let mut simulator = MockEventSimulator::new(temp_path.clone());
    
    // Configure watcher with larger debounce window for this test
    let config = WatcherConfig {
        debounce_ms: 200,  // 200ms debounce window
        batch_size: 100,
        max_queue_size: 1000,
        drain_timeout_ms: 1000,
    };
    
    let mut watcher = FileWatcher::new(vec![temp_path.clone()], config)
        .expect("Failed to create file watcher");
    
    // Start the watcher in a separate thread
    let watcher_thread = thread::spawn(move || {
        watcher.start().expect("Failed to start watcher");
        
        // Let it run for enough time to observe debouncing
        thread::sleep(Duration::from_secs(3));
        
        // Stop the watcher
        watcher.stop().expect("Failed to stop watcher");
        
        // Return the watcher for inspection
        watcher
    });
    
    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(500));
    
    // Rapidly update the same file multiple times within the debounce window
    let test_file = "test_debounce.txt";
    simulator.create_file(test_file, "Initial content");
    
    // Update the same file multiple times within debounce window
    for i in 1..10 {
        simulator.modify_file(test_file, &format!("Update {}", i));
        thread::sleep(Duration::from_millis(10)); // Short delay between updates
    }
    
    // Wait for debounce window and processing
    thread::sleep(Duration::from_millis(500));
    
    // Complete the test
    let finished_watcher = watcher_thread.join().expect("Watcher thread panicked");
    let status = finished_watcher.status().unwrap();
    
    // We should have processed fewer events than modifications made
    // due to debouncing (exact count depends on implementation details)
    assert!(status.events_processed < 10, 
            "Expected fewer events processed than modifications due to debouncing");
    
    // Clean up
    simulator.cleanup();
}

#[test]
fn test_event_flood() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_path_buf();
    
    let mut simulator = MockEventSimulator::new(temp_path.clone());
    
    // Configure with settings tuned for burst handling
    let config = WatcherConfig {
        debounce_ms: 100,
        batch_size: 500,  // Handle larger batches
        max_queue_size: 10000,  // Large queue for burst
        drain_timeout_ms: 5000, // Longer drain time for cleanup
    };
    
    let mut watcher = FileWatcher::new(vec![temp_path.clone()], config)
        .expect("Failed to create file watcher");
    
    // Start the watcher
    let watcher_thread = thread::spawn(move || {
        watcher.start().expect("Failed to start watcher");
        
        // Let it run for enough time to process a large burst
        thread::sleep(Duration::from_secs(10));
        
        // Stop the watcher
        watcher.stop().expect("Failed to stop watcher");
        
        // Return the watcher for inspection
        watcher
    });
    
    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(500));
    
    // Create 1000 files in rapid succession (smaller scale for test)
    let start_time = Instant::now();
    let created_files = simulator.create_burst(1000, "flood");
    let creation_time = start_time.elapsed();
    
    println!("Created 1000 files in {:?}", creation_time);
    
    // Wait for processing to complete
    thread::sleep(Duration::from_secs(5));
    
    // Complete the test
    let finished_watcher = watcher_thread.join().expect("Watcher thread panicked");
    let status = finished_watcher.status().unwrap();
    
    // Verify processing occurred
    assert!(status.events_processed > 0, "Expected events to be processed");
    assert_eq!(status.queue_size, 0, "Expected empty queue after stopping");
    
    // Clean up
    simulator.cleanup();
}

#[test]
fn test_hierarchical_debouncing() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_path_buf();
    
    let mut simulator = MockEventSimulator::new(temp_path.clone());
    
    // Configure watcher
    let config = WatcherConfig {
        debounce_ms: 200,
        batch_size: 100,
        max_queue_size: 1000,
        drain_timeout_ms: 1000,
    };
    
    let mut watcher = FileWatcher::new(vec![temp_path.clone()], config)
        .expect("Failed to create file watcher");
    
    // Start the watcher
    let watcher_thread = thread::spawn(move || {
        watcher.start().expect("Failed to start watcher");
        
        // Let it run
        thread::sleep(Duration::from_secs(5));
        
        // Stop the watcher
        watcher.stop().expect("Failed to stop watcher");
        
        // Return the watcher
        watcher
    });
    
    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(500));
    
    // Create directory structure
    let nested_dir = "parent/child/grandchild";
    fs::create_dir_all(temp_path.join(nested_dir)).expect("Failed to create nested directories");
    
    // Create files in the hierarchy
    simulator.create_file("parent/file1.txt", "Content 1");
    simulator.create_file("parent/child/file2.txt", "Content 2");
    simulator.create_file("parent/child/grandchild/file3.txt", "Content 3");
    
    // Wait a bit
    thread::sleep(Duration::from_millis(300));
    
    // Complete the test
    let finished_watcher = watcher_thread.join().expect("Watcher thread panicked");
    
    // Clean up
    simulator.cleanup();
}

#[test]
fn test_graceful_shutdown() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_path_buf();
    
    let mut simulator = MockEventSimulator::new(temp_path.clone());
    
    // Configure watcher with specific drain timeout
    let config = WatcherConfig {
        debounce_ms: 100,
        batch_size: 100,
        max_queue_size: 1000,
        drain_timeout_ms: 2000, // 2 second drain timeout
    };
    
    let mut watcher = FileWatcher::new(vec![temp_path.clone()], config)
        .expect("Failed to create file watcher");
    
    // Start the watcher
    watcher.start().expect("Failed to start watcher");
    
    // Wait for initialization
    thread::sleep(Duration::from_millis(500));
    
    // Create files
    for i in 0..10 {
        simulator.create_file(&format!("shutdown_test_{}.txt", i), "Shutdown test");
        thread::sleep(Duration::from_millis(10));
    }
    
    // Immediately request shutdown while events are being processed
    let shutdown_start = Instant::now();
    watcher.stop().expect("Failed to stop watcher");
    let shutdown_duration = shutdown_start.elapsed();
    
    // Shutdown should take close to the drain timeout but not excessively longer
    println!("Shutdown took {:?}", shutdown_duration);
    assert!(shutdown_duration >= Duration::from_millis(100), 
            "Shutdown was too quick, may not have drained properly");
    assert!(shutdown_duration <= Duration::from_millis(3000), 
            "Shutdown took too long");
    
    // Verify final state
    let status = watcher.status().unwrap();
    assert_eq!(status.state, WatcherState::Stopped);
    assert_eq!(status.queue_size, 0, "Queue should be empty after shutdown");
    
    // Clean up
    simulator.cleanup();
}
