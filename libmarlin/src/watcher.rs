//! File system watcher implementation for Marlin
//!
//! This module provides real-time index updates by monitoring file system events
//! (create, modify, delete) using the `notify` crate. It implements event debouncing,
//! batch processing, and a state machine for robust lifecycle management.

use anyhow::Result;
use crate::db::Database;
use crossbeam_channel::{bounded, Receiver};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
    /// The watcher is initializing
    Initializing,

    /// The watcher is actively monitoring file system events
    Watching,

    /// The watcher is paused (receiving but not processing events)
    Paused,

    /// The watcher is shutting down
    ShuttingDown,

    /// The watcher has stopped
    Stopped,
}

/// Status information about the file watcher
#[derive(Debug, Clone)]
pub struct WatcherStatus {
    /// Current state of the watcher
    pub state: WatcherState,

    /// Number of events processed since startup
    pub events_processed: usize,

    /// Current size of the event queue
    pub queue_size: usize,

    /// Time the watcher was started
    pub start_time: Option<Instant>,

    /// Paths being watched
    pub watched_paths: Vec<PathBuf>,
}

/// Priority levels for different types of events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EventPriority {
    /// File creation events (high priority)
    Create = 0,

    /// File deletion events (high priority)
    Delete = 1,

    /// File modification events (medium priority)
    Modify = 2,

    /// File access events (low priority)
    Access = 3,
}

/// Processed file system event with metadata
#[derive(Debug, Clone)]
struct ProcessedEvent {
    /// Path to the file or directory
    path: PathBuf,

    /// Type of event
    kind: EventKind,

    /// Priority of the event for processing order
    priority: EventPriority,

    /// Time the event was received
    timestamp: Instant,
}

/// Event debouncer for coalescing multiple events on the same file
struct EventDebouncer {
    /// Map of file paths to their latest events
    events: HashMap<PathBuf, ProcessedEvent>,

    /// Debounce window in milliseconds
    debounce_window_ms: u64,

    /// Last time the debouncer was flushed
    last_flush: Instant,
}

impl EventDebouncer {
    /// Create a new event debouncer with the specified debounce window
    fn new(debounce_window_ms: u64) -> Self {
        Self {
            events: HashMap::new(),
            debounce_window_ms,
            last_flush: Instant::now(),
        }
    }

    /// Add an event to the debouncer
    fn add_event(&mut self, event: ProcessedEvent) {
        let path = event.path.clone();

        // Apply hierarchical debouncing: directory events override contained files
        if path.is_dir() {
            self.events.retain(|file_path, _| !file_path.starts_with(&path));
        }

        // Update or insert the event for the file
        match self.events.get_mut(&path) {
            Some(existing) => {
                // Keep the higher priority event
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

    /// Check if the debouncer is ready to flush events
    fn is_ready_to_flush(&self) -> bool {
        self.last_flush.elapsed() >= Duration::from_millis(self.debounce_window_ms)
    }

    /// Flush all events, sorted by priority, and reset the debouncer
    fn flush(&mut self) -> Vec<ProcessedEvent> {
        let mut events: Vec<ProcessedEvent> = self.events.drain().map(|(_, e)| e).collect();
        events.sort_by_key(|e| e.priority);
        self.last_flush = Instant::now();
        events
    }

    /// Get the number of events in the debouncer
    #[allow(dead_code)] 
    fn len(&self) -> usize {
        self.events.len()
    }
}

/// Main file watcher implementation
pub struct FileWatcher {
    /// Current state of the watcher
    state: Arc<Mutex<WatcherState>>,

    /// Configuration for the watcher
    #[allow(dead_code)] 
    config: WatcherConfig,

    /// Paths being watched
    watched_paths: Vec<PathBuf>,

    /// Notify event receiver (original receiver, clone is used in thread)
    #[allow(dead_code)] 
    event_receiver: Receiver<std::result::Result<Event, notify::Error>>,

    /// Notify watcher instance (must be kept alive for watching to continue)
    #[allow(dead_code)] 
    watcher: RecommendedWatcher,

    /// Event processor thread
    processor_thread: Option<JoinHandle<()>>,

    /// Flag to signal the processor thread to stop
    stop_flag: Arc<AtomicBool>,

    /// Number of events processed
    events_processed: Arc<AtomicUsize>,

    /// Current queue size
    queue_size: Arc<AtomicUsize>,

    /// Start time of the watcher
    start_time: Instant,

    /// Optional database connection, shared with the processor thread.
    db_shared: Arc<Mutex<Option<Arc<Mutex<Database>>>>>,
}

impl FileWatcher {
    /// Create a new file watcher for the given paths
    pub fn new(paths: Vec<PathBuf>, config: WatcherConfig) -> Result<Self> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let events_processed = Arc::new(AtomicUsize::new(0));
        let queue_size = Arc::new(AtomicUsize::new(0));
        let state = Arc::new(Mutex::new(WatcherState::Initializing));

        let (tx, rx) = bounded(config.max_queue_size);
        
        let actual_watcher = notify::recommended_watcher(move |event_res| {
            if tx.send(event_res).is_err() {
                 // eprintln!("Watcher: Failed to send event to channel (receiver likely dropped)");
            }
        })?;

        let mut mutable_watcher_ref = actual_watcher;
        for path in &paths {
            mutable_watcher_ref.watch(path, RecursiveMode::Recursive)?;
        }

        let config_clone = config.clone(); 
        let stop_flag_clone = stop_flag.clone();
        let events_processed_clone = events_processed.clone();
        let queue_size_clone = queue_size.clone();
        let state_clone = state.clone();
        let receiver_clone = rx.clone(); 
        
        // Correct initialization: Mutex protecting an Option, which starts as None.
        let db_shared_for_thread = Arc::new(Mutex::new(None::<Arc<Mutex<Database>>>));
        let db_captured_for_thread = db_shared_for_thread.clone();


        let processor_thread = thread::spawn(move || {
            let mut debouncer = EventDebouncer::new(config_clone.debounce_ms);
            
            while !stop_flag_clone.load(Ordering::SeqCst) {
                {
                    let state_guard = state_clone.lock().unwrap();
                    if *state_guard == WatcherState::Paused {
                        drop(state_guard); 
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                } 

                while let Ok(evt_res) = receiver_clone.try_recv() { 
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
                        Err(e) => eprintln!("Watcher channel error: {:?}", e),
                    }
                }

                queue_size_clone.store(debouncer.len(), Ordering::SeqCst);

                if debouncer.is_ready_to_flush() && debouncer.len() > 0 {
                    let evts = debouncer.flush();
                    let num_evts = evts.len();
                    events_processed_clone.fetch_add(num_evts, Ordering::SeqCst);
                    
                    let db_opt_arc_guard = db_captured_for_thread.lock().unwrap();
                    if let Some(db_arc) = &*db_opt_arc_guard { 
                        let _db_guard = db_arc.lock().unwrap(); 
                        for event in &evts {
                             println!("Processing event (DB available): {:?} for path {:?}", event.kind, event.path);
                        }
                    } else {
                        for event in &evts {
                            println!("Processing event (no DB): {:?} for path {:?}", event.kind, event.path);
                        }
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }

            if debouncer.len() > 0 {
                let evts = debouncer.flush();
                events_processed_clone.fetch_add(evts.len(), Ordering::SeqCst);
                 for processed_event in evts {
                         println!("Processing final event: {:?} for path {:?}", processed_event.kind, processed_event.path);
                    }
            }

            let mut state_guard = state_clone.lock().unwrap();
            *state_guard = WatcherState::Stopped;
        });

        let watcher_instance = Self {
            state,
            config,
            watched_paths: paths,
            event_receiver: rx, 
            watcher: mutable_watcher_ref,
            processor_thread: Some(processor_thread),
            stop_flag,
            events_processed,
            queue_size,
            start_time: Instant::now(),
            db_shared: db_shared_for_thread, 
        };
        Ok(watcher_instance)
    }

    /// Set the database connection for the watcher.
    pub fn with_database(&mut self, db_arc: Arc<Mutex<Database>>) -> &mut Self {
        {
            let mut shared_db_guard = self.db_shared.lock().unwrap();
            *shared_db_guard = Some(db_arc);
        } 
        self
    }

    /// Start the file watcher.
    pub fn start(&mut self) -> Result<()> {
        let mut state_guard = self.state.lock().unwrap();
        if *state_guard == WatcherState::Watching || (*state_guard == WatcherState::Initializing && self.processor_thread.is_some()) {
             if *state_guard == WatcherState::Initializing {
                *state_guard = WatcherState::Watching;
             }
             return Ok(()); 
        }
        *state_guard = WatcherState::Watching;
        Ok(())
    }

    /// Pause the watcher.
    pub fn pause(&mut self) -> Result<()> {
        let mut state_guard = self.state.lock().unwrap();
        match *state_guard {
            WatcherState::Watching => {
                *state_guard = WatcherState::Paused;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Watcher not in watching state to pause")),
        }
    }

    /// Resume a paused watcher.
    pub fn resume(&mut self) -> Result<()> {
        let mut state_guard = self.state.lock().unwrap();
        match *state_guard {
            WatcherState::Paused => {
                *state_guard = WatcherState::Watching;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Watcher not in paused state to resume")),
        }
    }

    /// Stop the watcher.
    pub fn stop(&mut self) -> Result<()> {
        let mut state_guard = self.state.lock().unwrap();
        if *state_guard == WatcherState::Stopped || *state_guard == WatcherState::ShuttingDown {
            return Ok(());
        }
        *state_guard = WatcherState::ShuttingDown;
        drop(state_guard); 

        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.processor_thread.take() {
            match handle.join() {
                Ok(_) => (), 
                Err(e) => eprintln!("Failed to join processor thread: {:?}", e),
            }
        }
        
        let mut final_state_guard = self.state.lock().unwrap();
        *final_state_guard = WatcherState::Stopped;
        Ok(())
    }

    /// Get the current status of the watcher.
    pub fn status(&self) -> WatcherStatus {
        let state_guard = self.state.lock().unwrap().clone();
        WatcherStatus {
            state: state_guard,
            events_processed: self.events_processed.load(Ordering::SeqCst),
            queue_size: self.queue_size.load(Ordering::SeqCst),
            start_time: Some(self.start_time),
            watched_paths: self.watched_paths.clone(),
        }
    }
}

impl Drop for FileWatcher {
    /// Ensure the watcher is stopped when dropped to prevent resource leaks.
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            eprintln!("Error stopping watcher in Drop: {:?}", e);
        }
    }
}