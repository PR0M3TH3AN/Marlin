// libmarlin/src/watcher.rs

//! File system watcher implementation for Marlin
//!
//! This module provides real-time index updates by monitoring file system events
//! (create, modify, delete) using the `notify` crate. It implements event debouncing,
//! batch processing, and a state machine for robust lifecycle management.

use anyhow::{Context, Result};
use crate::db::Database;
use crossbeam_channel::{bounded, Receiver};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcherTrait};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::info;

/// Configuration for the file watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Time in milliseconds to debounce file events
    pub debounce_ms: u64,

    /// Maximum number of events to process in a single batch
    pub batch_size: usize,

    /// Maximum size of the event queue before applying backpressure
    pub max_queue_size: usize,

    /// Time in milliseconds to wait for events to drain during shutdown
    pub drain_timeout_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            batch_size: 1000,
            max_queue_size: 100_000,
            drain_timeout_ms: 5000,
        }
    }
}

/// State of the file watcher
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherState {
    Initializing,
    Watching,
    Paused,
    ShuttingDown,
    Stopped,
}

/// Status information about the file watcher
#[derive(Debug, Clone)]
pub struct WatcherStatus {
    pub state: WatcherState,
    pub events_processed: usize,
    pub queue_size: usize,
    pub start_time: Option<Instant>,
    pub watched_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EventPriority {
    Create = 0,
    Delete = 1,
    Modify = 2,
    Access = 3,
}

#[derive(Debug, Clone)]
struct ProcessedEvent {
    path: PathBuf,
    kind: EventKind,
    priority: EventPriority,
    timestamp: Instant,
}

struct EventDebouncer {
    events: HashMap<PathBuf, ProcessedEvent>,
    debounce_window_ms: u64,
    last_flush: Instant,
}

impl EventDebouncer {
    fn new(debounce_window_ms: u64) -> Self {
        Self {
            events: HashMap::new(),
            debounce_window_ms,
            last_flush: Instant::now(),
        }
    }

    fn add_event(&mut self, event: ProcessedEvent) {
        let path = event.path.clone();
        if path.is_dir() { // This relies on the PathBuf itself knowing if it's a directory
                           // or on the underlying FS. For unit tests, ensure paths are created.
            self.events.retain(|file_path, _| !file_path.starts_with(&path) || file_path == &path );
        }
        match self.events.get_mut(&path) {
            Some(existing) => {
                if event.priority < existing.priority {
                    existing.priority = event.priority;
                }
                existing.timestamp = event.timestamp;
                existing.kind = event.kind;
            }
            None => {
                self.events.insert(path, event);
            }
        }
    }

    fn is_ready_to_flush(&self) -> bool {
        self.last_flush.elapsed() >= Duration::from_millis(self.debounce_window_ms)
    }

    fn flush(&mut self) -> Vec<ProcessedEvent> {
        let mut events: Vec<ProcessedEvent> = self.events.drain().map(|(_, e)| e).collect();
        events.sort_by_key(|e| e.priority);
        self.last_flush = Instant::now();
        events
    }

    fn len(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod event_debouncer_tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode};
    use std::fs; // fs is needed for these tests to create dirs/files
    use tempfile; 

    #[test]
    fn debouncer_add_and_flush() {
        let mut debouncer = EventDebouncer::new(100);
        std::thread::sleep(Duration::from_millis(110)); 
        assert!(debouncer.is_ready_to_flush());
        assert_eq!(debouncer.len(), 0);

        let path1 = PathBuf::from("file1.txt");
        debouncer.add_event(ProcessedEvent {
            path: path1.clone(),
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        assert_eq!(debouncer.len(), 1);
        
        debouncer.last_flush = Instant::now(); 
        assert!(!debouncer.is_ready_to_flush());

        std::thread::sleep(Duration::from_millis(110));
        assert!(debouncer.is_ready_to_flush());

        let flushed = debouncer.flush();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].path, path1);
        assert_eq!(debouncer.len(), 0);
        assert!(!debouncer.is_ready_to_flush()); 
    }

    #[test]
    fn debouncer_coalesce_events() {
        let mut debouncer = EventDebouncer::new(100);
        let path1 = PathBuf::from("file1.txt");

        let t1 = Instant::now();
        debouncer.add_event(ProcessedEvent {
            path: path1.clone(),
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: t1,
        });
        std::thread::sleep(Duration::from_millis(10));
        let t2 = Instant::now();
        debouncer.add_event(ProcessedEvent {
            path: path1.clone(),
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            priority: EventPriority::Modify,
            timestamp: t2,
        });
        
        assert_eq!(debouncer.len(), 1);
        
        std::thread::sleep(Duration::from_millis(110));
        let flushed = debouncer.flush();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].path, path1);
        assert_eq!(flushed[0].priority, EventPriority::Create); 
        assert_eq!( 
            flushed[0].kind,
            EventKind::Modify(ModifyKind::Data(DataChange::Any))
        );
        assert_eq!(flushed[0].timestamp, t2);
    }

    #[test]
    fn debouncer_hierarchical() {
        let mut debouncer_h = EventDebouncer::new(100);
        let temp_dir_obj = tempfile::tempdir().expect("Failed to create temp dir");
        let p_dir = temp_dir_obj.path().to_path_buf(); 
        let p_file = p_dir.join("file.txt");
        
        fs::File::create(&p_file).expect("Failed to create test file for hierarchical debounce");

        debouncer_h.add_event(ProcessedEvent {
            path: p_file.clone(),
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        assert_eq!(debouncer_h.len(), 1);
        
        debouncer_h.add_event(ProcessedEvent {
            path: p_dir.clone(), 
            kind: EventKind::Remove(RemoveKind::Folder), 
            priority: EventPriority::Delete,
            timestamp: Instant::now(),
        });
        assert_eq!(debouncer_h.len(), 1, "Hierarchical debounce should remove child event, leaving only parent dir event");
        
        std::thread::sleep(Duration::from_millis(110));
        let flushed = debouncer_h.flush();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].path, p_dir);
    }

    #[test]
    fn debouncer_different_files() {
        let mut debouncer = EventDebouncer::new(100);
        let path1 = PathBuf::from("file1.txt");
        let path2 = PathBuf::from("file2.txt");

        debouncer.add_event(ProcessedEvent {
            path: path1.clone(),
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        debouncer.add_event(ProcessedEvent {
            path: path2.clone(),
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        assert_eq!(debouncer.len(), 2);
        std::thread::sleep(Duration::from_millis(110));
        let flushed = debouncer.flush();
        assert_eq!(flushed.len(), 2);
    }

    #[test]
    fn debouncer_priority_sorting_on_flush() {
        let mut debouncer = EventDebouncer::new(100);
        let path1 = PathBuf::from("file1.txt"); 
        let path2 = PathBuf::from("file2.txt"); 
        let path3 = PathBuf::from("file3.txt"); 

        debouncer.add_event(ProcessedEvent { path: path1, kind: EventKind::Modify(ModifyKind::Name(RenameMode::To)), priority: EventPriority::Modify, timestamp: Instant::now() });
        debouncer.add_event(ProcessedEvent { path: path2, kind: EventKind::Create(CreateKind::File), priority: EventPriority::Create, timestamp: Instant::now() });
        debouncer.add_event(ProcessedEvent { path: path3, kind: EventKind::Remove(RemoveKind::File), priority: EventPriority::Delete, timestamp: Instant::now() });
        
        std::thread::sleep(Duration::from_millis(110));
        let flushed = debouncer.flush();
        assert_eq!(flushed.len(), 3);
        assert_eq!(flushed[0].priority, EventPriority::Create); 
        assert_eq!(flushed[1].priority, EventPriority::Delete); 
        assert_eq!(flushed[2].priority, EventPriority::Modify); 
    }

    #[test]
    fn debouncer_no_events_flush_empty() {
        let mut debouncer = EventDebouncer::new(100);
        std::thread::sleep(Duration::from_millis(110));
        let flushed = debouncer.flush();
        assert!(flushed.is_empty());
        assert_eq!(debouncer.len(), 0);
    }

    #[test]
    fn debouncer_dir_then_file_hierarchical() {
        let mut debouncer = EventDebouncer::new(100);
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let dir = temp_dir.path().to_path_buf();
        let file = dir.join("child.txt");

        debouncer.add_event(ProcessedEvent {
            path: dir.clone(),
            kind: EventKind::Create(CreateKind::Folder),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        debouncer.add_event(ProcessedEvent {
            path: file,
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });

        assert_eq!(debouncer.len(), 2);
        std::thread::sleep(Duration::from_millis(110));
        let flushed = debouncer.flush();
        assert_eq!(flushed.len(), 2);
        assert!(flushed.iter().any(|e| e.path == dir));
    }
}


pub struct FileWatcher {
    state: Arc<Mutex<WatcherState>>,
    _config: WatcherConfig,
    watched_paths: Vec<PathBuf>,
    _event_receiver: Receiver<std::result::Result<Event, notify::Error>>,
    _watcher: RecommendedWatcher,
    processor_thread: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    events_processed: Arc<AtomicUsize>,
    queue_size: Arc<AtomicUsize>,
    start_time: Instant,
    db_shared: Arc<Mutex<Option<Arc<Mutex<Database>>>>>,
}

impl FileWatcher {
    pub fn new(paths: Vec<PathBuf>, config: WatcherConfig) -> Result<Self> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let events_processed = Arc::new(AtomicUsize::new(0));
        let queue_size = Arc::new(AtomicUsize::new(0));
        let state = Arc::new(Mutex::new(WatcherState::Initializing));

        let (tx, rx) = bounded(config.max_queue_size);

        let event_tx = tx.clone();
        let mut actual_watcher = RecommendedWatcher::new(
            move |event_res: std::result::Result<Event, notify::Error>| {
                if event_tx.send(event_res).is_err() {
                    // Receiver dropped
                }
            },
            notify::Config::default(),
        )?;

        for path_to_watch in &paths {
            actual_watcher
                .watch(path_to_watch, RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch path: {}", path_to_watch.display()))?;
        }

        let config_clone = config.clone();
        let stop_flag_clone = stop_flag.clone();
        let events_processed_clone = events_processed.clone();
        let queue_size_clone = queue_size.clone();
        let state_clone = state.clone();
        let receiver_clone = rx.clone(); 

        let db_shared_for_thread = Arc::new(Mutex::new(None::<Arc<Mutex<Database>>>));
        let db_captured_for_thread = db_shared_for_thread.clone();

        let processor_thread = thread::spawn(move || {
            let mut debouncer = EventDebouncer::new(config_clone.debounce_ms);

            while !stop_flag_clone.load(Ordering::Relaxed) { 
                let current_state = match state_clone.lock() {
                    Ok(g) => g.clone(),
                    Err(_) => {
                        eprintln!("state mutex poisoned");
                        break;
                    }
                };

                if current_state == WatcherState::Paused {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                if current_state == WatcherState::ShuttingDown || current_state == WatcherState::Stopped {
                    break;
                }

                let mut received_in_batch = 0;
                while let Ok(evt_res) = receiver_clone.try_recv() {
                    received_in_batch +=1;
                    match evt_res {
                        Ok(event) => {
                            for path in event.paths {
                                let prio = match event.kind {
                                    EventKind::Create(_) => EventPriority::Create,
                                    EventKind::Remove(_) => EventPriority::Delete,
                                    EventKind::Modify(_) => EventPriority::Modify,
                                    EventKind::Access(_) => EventPriority::Access,
                                    _ => EventPriority::Modify,
                                };
                                debouncer.add_event(ProcessedEvent {
                                    path,
                                    kind: event.kind.clone(),
                                    priority: prio,
                                    timestamp: Instant::now(),
                                });
                            }
                        }
                        Err(e) => {
                            eprintln!("Watcher channel error: {:?}", e);
                        }
                    }
                    if received_in_batch >= config_clone.batch_size {
                        break;
                    }
                }

                queue_size_clone.store(debouncer.len(), Ordering::SeqCst);

                if debouncer.is_ready_to_flush() && debouncer.len() > 0 {
                    let evts_to_process = debouncer.flush();
                    let num_evts = evts_to_process.len();
                    events_processed_clone.fetch_add(num_evts, Ordering::SeqCst);

                    let db_guard_option = match db_captured_for_thread.lock() {
                        Ok(g) => g,
                        Err(_) => {
                            eprintln!("db_shared mutex poisoned");
                            break;
                        }
                    };
                    if let Some(db_mutex) = &*db_guard_option {
                        if let Ok(mut _db_instance_guard) = db_mutex.lock() {
                            for event_item in &evts_to_process {
                                info!(?event_item.kind, path = ?event_item.path, "Processing event (DB available)");
                            }
                        } else {
                            eprintln!("db mutex poisoned");
                        }
                    } else {
                        for event_item in &evts_to_process {
                            info!(?event_item.kind, path = ?event_item.path, "Processing event (no DB)");
                        }
                    }
                }
                thread::sleep(Duration::from_millis(50));
            }

            if debouncer.len() > 0 {
                let final_evts = debouncer.flush();
                events_processed_clone.fetch_add(final_evts.len(), Ordering::SeqCst);
                for processed_event in final_evts {
                    info!(?processed_event.kind, path = ?processed_event.path, "Processing final event");
                }
            }
            if let Ok(mut final_state_guard) = state_clone.lock() {
                *final_state_guard = WatcherState::Stopped;
            } else {
                eprintln!("state mutex poisoned on shutdown");
            }
        });

        Ok(Self {
            state,
            _config: config,
            watched_paths: paths,
            _event_receiver: rx,
            _watcher: actual_watcher,
            processor_thread: Some(processor_thread),
            stop_flag,
            events_processed,
            queue_size,
            start_time: Instant::now(),
            db_shared: db_shared_for_thread,
        })
    }

    pub fn with_database(&mut self, db_arc: Arc<Mutex<Database>>) -> Result<&mut Self> {
        {
            let mut shared_db_guard = self
                .db_shared
                .lock()
                .map_err(|_| anyhow::anyhow!("db_shared mutex poisoned"))?;
            *shared_db_guard = Some(db_arc);
        }
        Ok(self)
    }

    pub fn start(&mut self) -> Result<()> {
        let mut state_guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state mutex poisoned"))?;
        if *state_guard == WatcherState::Watching || self.processor_thread.is_none() {
            if self.processor_thread.is_none() {
                return Err(anyhow::anyhow!("Watcher thread not available to start."));
            }
            if *state_guard == WatcherState::Initializing {
                 *state_guard = WatcherState::Watching;
            }
            return Ok(());
        }
        if *state_guard != WatcherState::Initializing && *state_guard != WatcherState::Stopped && *state_guard != WatcherState::Paused {
            return Err(anyhow::anyhow!(format!("Cannot start watcher from state {:?}", *state_guard)));
        }

        *state_guard = WatcherState::Watching;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        let mut state_guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state mutex poisoned"))?;
        match *state_guard {
            WatcherState::Watching => {
                *state_guard = WatcherState::Paused;
                Ok(())
            }
            WatcherState::Paused => Ok(()), 
            _ => Err(anyhow::anyhow!(format!("Watcher not in watching state to pause (current: {:?})", *state_guard))),
        }
    }

    pub fn resume(&mut self) -> Result<()> {
        let mut state_guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state mutex poisoned"))?;
        match *state_guard {
            WatcherState::Paused => {
                *state_guard = WatcherState::Watching;
                Ok(())
            }
            WatcherState::Watching => Ok(()), 
            _ => Err(anyhow::anyhow!(format!("Watcher not in paused state to resume (current: {:?})", *state_guard))),
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        let mut current_state_guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state mutex poisoned"))?;
        if *current_state_guard == WatcherState::Stopped || *current_state_guard == WatcherState::ShuttingDown {
            return Ok(());
        }
        *current_state_guard = WatcherState::ShuttingDown;
        drop(current_state_guard);

        self.stop_flag.store(true, Ordering::SeqCst);

        if let Some(handle) = self.processor_thread.take() {
            match handle.join() {
                Ok(_) => { /* Thread joined cleanly */ }
                Err(join_err) => {
                    eprintln!("Watcher processor thread panicked: {:?}", join_err);
                }
            }
        }
        
        let mut final_state_guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state mutex poisoned"))?;
        *final_state_guard = WatcherState::Stopped;
        Ok(())
    }

    pub fn status(&self) -> Result<WatcherStatus> {
        let state_guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state mutex poisoned"))?
            .clone();
        Ok(WatcherStatus {
            state: state_guard,
            events_processed: self.events_processed.load(Ordering::SeqCst),
            queue_size: self.queue_size.load(Ordering::SeqCst),
            start_time: Some(self.start_time),
            watched_paths: self.watched_paths.clone(),
        })
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            eprintln!("Error stopping watcher in Drop: {:?}", e);
        }
    }
}


#[cfg(test)]
mod file_watcher_state_tests { 
    use super::*;
    use tempfile::tempdir;
    use std::fs as FsMod; // Alias to avoid conflict with local `fs` module name if any

    #[test]
    fn test_watcher_pause_resume_stop() {
        let tmp_dir = tempdir().unwrap();
        let watch_path = tmp_dir.path().to_path_buf();
        FsMod::create_dir_all(&watch_path).expect("Failed to create temp dir for watching");

        let config = WatcherConfig::default();

        let mut watcher = FileWatcher::new(vec![watch_path], config).expect("Failed to create watcher");

        assert_eq!(watcher.status().unwrap().state, WatcherState::Initializing);

        watcher.start().expect("Start failed");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Watching);

        watcher.pause().expect("Pause failed");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Paused);

        watcher.pause().expect("Second pause failed");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Paused);

        watcher.resume().expect("Resume failed");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Watching);
        
        watcher.resume().expect("Second resume failed");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Watching);

        watcher.stop().expect("Stop failed");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Stopped);

        watcher.stop().expect("Second stop failed");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Stopped);
    }

    #[test]
    fn test_watcher_start_errors() {
        let tmp_dir = tempdir().unwrap();
        FsMod::create_dir_all(tmp_dir.path()).expect("Failed to create temp dir for watching");
        let mut watcher = FileWatcher::new(vec![tmp_dir.path().to_path_buf()], WatcherConfig::default()).unwrap();
        
        {
            let mut state_guard = watcher
                .state
                .lock()
                .expect("state mutex poisoned");
            *state_guard = WatcherState::Watching;
        }
        assert!(watcher.start().is_ok(), "Should be able to call start when already Watching (idempotent state change)");
        assert_eq!(watcher.status().unwrap().state, WatcherState::Watching);
        
        {
             let mut state_guard = watcher
                .state
                .lock()
                .expect("state mutex poisoned");
            *state_guard = WatcherState::ShuttingDown;
        }
        assert!(watcher.start().is_err(), "Should not be able to start from ShuttingDown");
    }

     #[test]
    fn test_new_watcher_with_nonexistent_path() {
        let non_existent_path = PathBuf::from("/path/that/REALLY/does/not/exist/for/sure/and/cannot/be/created");
        let config = WatcherConfig::default();
        let watcher_result = FileWatcher::new(vec![non_existent_path], config);
        assert!(watcher_result.is_err());
        if let Err(e) = watcher_result {
            let err_string = e.to_string();
            assert!(err_string.contains("Failed to watch path") || err_string.contains("os error 2"), "Error was: {}", err_string);
        }
    }

    #[test]
    fn test_watcher_default_config() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.max_queue_size, 100_000);
        assert_eq!(config.drain_timeout_ms, 5000);
    }

    #[test]
    fn test_poisoned_state_mutex_errors() {
        let tmp_dir = tempdir().unwrap();
        let watch_path = tmp_dir.path().to_path_buf();
        FsMod::create_dir_all(&watch_path).expect("Failed to create temp dir for watching");

        let config = WatcherConfig::default();

        let mut watcher = FileWatcher::new(vec![watch_path], config).expect("Failed to create watcher");

        let state_arc = watcher.state.clone();
        let _ = std::thread::spawn(move || {
            let _guard = state_arc.lock().unwrap();
            panic!("poison");
        })
        .join();

        assert!(watcher.start().is_err());
        assert!(watcher.pause().is_err());
        assert!(watcher.resume().is_err());
        assert!(watcher.stop().is_err());
        assert!(watcher.status().is_err());
    }
}