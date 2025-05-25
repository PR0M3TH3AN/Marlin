//! File system watcher implementation for Marlin
//!
//! This module provides real-time index updates by monitoring file-system
//! events (create/modify/delete/rename) using the `notify` crate.  It adds
//! event-debouncing, batch processing and a small state-machine so that the
//! watcher can be paused, resumed and shut down cleanly.

use crate::db::{self, Database};
use crate::utils::to_db_path;
use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{bounded, Receiver};
use notify::{
    event::{ModifyKind, RemoveKind, RenameMode},
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcherTrait,
};
use same_file::Handle;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::info;

// ────── configuration ─────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub debounce_ms: u64,
    pub batch_size: usize,
    pub max_queue_size: usize,
    pub drain_timeout_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            batch_size: 1_000,
            max_queue_size: 100_000,
            drain_timeout_ms: 5_000,
        }
    }
}

// ────── public state/useful telemetry ────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherState {
    Initializing,
    Watching,
    Paused,
    ShuttingDown,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct WatcherStatus {
    pub state: WatcherState,
    pub events_processed: usize,
    pub queue_size: usize,
    pub start_time: Option<Instant>,
    pub watched_paths: Vec<PathBuf>,
}

// ────── internal bookkeeping ─────────────────────────────────────────────────
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
    old_path: Option<PathBuf>,
    new_path: Option<PathBuf>,
    kind: EventKind,
    priority: EventPriority,
    timestamp: Instant,
}

struct EventDebouncer {
    events: HashMap<PathBuf, ProcessedEvent>,
    debounce_window_ms: u64,
    last_flush: Instant,
}

#[cfg(any(target_os = "redox", unix))]
fn handle_key(h: &Handle) -> u64 {
    h.ino()
}

#[cfg(not(any(target_os = "redox", unix)))]
fn handle_key(h: &Handle) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    h.hash(&mut hasher);
    hasher.finish()
}

#[derive(Default)]
struct RemoveTracker {
    map: HashMap<u64, (PathBuf, Instant)>,
}

impl RemoveTracker {
    fn record(&mut self, path: &PathBuf) {
        if let Ok(h) = Handle::from_path(path) {
            self.map
                .insert(handle_key(&h), (path.clone(), Instant::now()));
            return;
        }

        // fall back to hashing path if handle could not be obtained
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        self.map
            .insert(hasher.finish(), (path.clone(), Instant::now()));
    }

    fn match_create(&mut self, path: &PathBuf, window: Duration) -> Option<PathBuf> {
        if let Ok(h) = Handle::from_path(path) {
            if let Some((old, ts)) = self.map.remove(&handle_key(&h)) {
                if Instant::now().duration_since(ts) <= window {
                    return Some(old);
                } else {
                    return None;
                }
            }
        }

        // fall back to hashing path when handle not available
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        if let Some((old, ts)) = self.map.remove(&hasher.finish()) {
            if Instant::now().duration_since(ts) <= window {
                return Some(old);
            } else {
                return None;
            }
        }
        None
    }

    fn flush_expired(&mut self, window: Duration, debouncer: &mut EventDebouncer) {
        let now = Instant::now();
        let mut expired = Vec::new();
        for (key, (path, ts)) in &self.map {
            if now.duration_since(*ts) > window {
                debouncer.add_event(ProcessedEvent {
                    path: path.clone(),
                    old_path: None,
                    new_path: None,
                    kind: EventKind::Remove(RemoveKind::Any),
                    priority: EventPriority::Delete,
                    timestamp: *ts,
                });
                expired.push(*key);
            }
        }
        for key in expired {
            self.map.remove(&key);
        }
    }
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

        // If we receive an event for a directory, purge any queued events under it
        if path.is_dir() {
            self.events
                .retain(|p, _| !p.starts_with(&path) || p == &path);
        }

        use std::collections::hash_map::Entry;

        match self.events.entry(path) {
            Entry::Occupied(mut o) => {
                let existing = o.get_mut();
                if event.priority < existing.priority {
                    existing.priority = event.priority;
                }
                existing.kind = event.kind;
                existing.timestamp = event.timestamp;
                if let Some(old_p) = event.old_path {
                    existing.old_path = Some(old_p);
                }
                if let Some(new_p) = event.new_path {
                    existing.new_path = Some(new_p);
                }
            }
            Entry::Vacant(v) => {
                v.insert(event);
            }
        }
    }

    fn is_ready_to_flush(&self) -> bool {
        self.last_flush.elapsed() >= Duration::from_millis(self.debounce_window_ms)
    }

    fn flush(&mut self) -> Vec<ProcessedEvent> {
        let mut v: Vec<_> = self.events.drain().map(|(_, e)| e).collect();
        v.sort_by_key(|e| e.priority);
        self.last_flush = Instant::now();
        v
    }

    fn len(&self) -> usize {
        self.events.len()
    }
}

// ────── main watcher struct ───────────────────────────────────────────────────
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
        // ── basic shared state/channels ───────────────────────────────────────
        let stop_flag = Arc::new(AtomicBool::new(false));
        let events_processed = Arc::new(AtomicUsize::new(0));
        let queue_size = Arc::new(AtomicUsize::new(0));
        let state = Arc::new(Mutex::new(WatcherState::Initializing));

        let (tx, rx) = bounded(config.max_queue_size);

        // ── start actual OS watcher ───────────────────────────────────────────
        let event_tx = tx.clone();
        let mut actual_watcher = RecommendedWatcher::new(
            move |ev| {
                let _ = event_tx.send(ev);
            },
            notify::Config::default(),
        )?;

        for p in &paths {
            actual_watcher
                .watch(p, RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch path {}", p.display()))?;
        }

        // ── spawn processor thread ────────────────────────────────────────────
        let config_clone = config.clone();
        let stop_flag_clone = stop_flag.clone();
        let events_processed_clone = events_processed.clone();
        let queue_size_clone = queue_size.clone();
        let state_clone = state.clone();
        let receiver_clone = rx.clone();

        let db_shared_for_thread: Arc<Mutex<Option<Arc<Mutex<Database>>>>> =
            Arc::new(Mutex::new(None));
        let db_for_thread = db_shared_for_thread.clone();

        fn handle_db_update(
            db_mutex: &Mutex<Database>,
            old_s: &str,
            new_s: &str,
            is_dir: bool,
        ) -> Result<()> {
            let mut guard = db_mutex.lock().map_err(|_| anyhow!("db mutex poisoned"))?;
            if is_dir {
                db::rename_directory(guard.conn_mut(), old_s, new_s)?;
            } else {
                db::update_file_path(guard.conn_mut(), old_s, new_s)?;
            }
            Ok(())
        }

        let processor_thread = thread::spawn(move || {
            let mut debouncer = EventDebouncer::new(config_clone.debounce_ms);
            let mut rename_cache: HashMap<usize, PathBuf> = HashMap::new();
            let mut remove_tracker = RemoveTracker::default();

            while !stop_flag_clone.load(Ordering::Relaxed) {
                // honour current state
                let cur_state = {
                    match state_clone.lock() {
                        Ok(g) => g.clone(),
                        Err(_) => break,
                    }
                };

                match cur_state {
                    WatcherState::Paused | WatcherState::Initializing => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    WatcherState::ShuttingDown | WatcherState::Stopped => break,
                    WatcherState::Watching => {} // normal path
                }

                // ── drain events (bounded by batch_size) ─────────────────────
                let mut processed_in_batch = 0;
                while let Ok(evt_res) = receiver_clone.try_recv() {
                    processed_in_batch += 1;
                    match evt_res {
                        Ok(event) => {
                            let prio = match event.kind {
                                EventKind::Create(_) => EventPriority::Create,
                                EventKind::Remove(_) => EventPriority::Delete,
                                EventKind::Modify(_) => EventPriority::Modify,
                                EventKind::Access(_) => EventPriority::Access,
                                _ => EventPriority::Modify,
                            };

                            // ── per-event logic ───────────────────────────────
                            match event.kind {
                                // 1. remove-then-create → rename heuristic using inode
                                EventKind::Remove(_) if event.paths.len() == 1 => {
                                    remove_tracker.record(&event.paths[0]);
                                }

                                EventKind::Create(_) if event.paths.len() == 1 => {
                                    if let Some(old_p) = remove_tracker
                                        .match_create(&event.paths[0], Duration::from_millis(500))
                                    {
                                        let new_p = event.paths[0].clone();
                                        debouncer.add_event(ProcessedEvent {
                                            path: old_p.clone(),
                                            old_path: Some(old_p),
                                            new_path: Some(new_p),
                                            kind: EventKind::Modify(ModifyKind::Name(
                                                RenameMode::Both,
                                            )),
                                            priority: prio,
                                            timestamp: Instant::now(),
                                        });
                                        continue;
                                    }

                                    for p in event.paths {
                                        debouncer.add_event(ProcessedEvent {
                                            path: p,
                                            old_path: None,
                                            new_path: None,
                                            kind: event.kind,
                                            priority: prio,
                                            timestamp: Instant::now(),
                                        });
                                    }
                                }

                                // 2. native rename events from notify
                                EventKind::Modify(ModifyKind::Name(name_kind)) => match name_kind {
                                    // Notify >= 6 emits `Both` when both paths are
                                    // supplied and `Any` as a catch-all for renames.
                                    // Treat both cases as a complete rename.
                                    RenameMode::Both | RenameMode::Any => {
                                        if event.paths.len() >= 2 {
                                            let old_p = event.paths[0].clone();
                                            let new_p = event.paths[1].clone();
                                            debouncer.add_event(ProcessedEvent {
                                                path: old_p.clone(),
                                                old_path: Some(old_p),
                                                new_path: Some(new_p),
                                                kind: EventKind::Modify(ModifyKind::Name(
                                                    RenameMode::Both,
                                                )),
                                                priority: prio,
                                                timestamp: Instant::now(),
                                            });
                                        }
                                    }
                                    RenameMode::From => {
                                        if let (Some(trk), Some(p)) =
                                            (event.tracker(), event.paths.first())
                                        {
                                            rename_cache.insert(trk, p.clone());
                                        }
                                        for p in event.paths {
                                            debouncer.add_event(ProcessedEvent {
                                                path: p,
                                                old_path: None,
                                                new_path: None,
                                                kind: event.kind,
                                                priority: prio,
                                                timestamp: Instant::now(),
                                            });
                                        }
                                    }
                                    RenameMode::To => {
                                        if let (Some(trk), Some(new_p)) =
                                            (event.tracker(), event.paths.first())
                                        {
                                            if let Some(old_p) = rename_cache.remove(&trk) {
                                                debouncer.add_event(ProcessedEvent {
                                                    path: old_p.clone(),
                                                    old_path: Some(old_p),
                                                    new_path: Some(new_p.clone()),
                                                    kind: EventKind::Modify(ModifyKind::Name(
                                                        RenameMode::Both,
                                                    )),
                                                    priority: prio,
                                                    timestamp: Instant::now(),
                                                });
                                                continue;
                                            }
                                        }
                                        for p in event.paths {
                                            debouncer.add_event(ProcessedEvent {
                                                path: p,
                                                old_path: None,
                                                new_path: None,
                                                kind: event.kind,
                                                priority: prio,
                                                timestamp: Instant::now(),
                                            });
                                        }
                                    }
                                    // `From`/`To` are handled above. Any other
                                    // value (`Other` or legacy `Rename`/`Move`
                                    // variants) is treated as a normal modify
                                    // event.
                                    _ => {
                                        for p in event.paths {
                                            debouncer.add_event(ProcessedEvent {
                                                path: p,
                                                old_path: None,
                                                new_path: None,
                                                kind: event.kind,
                                                priority: prio,
                                                timestamp: Instant::now(),
                                            });
                                        }
                                    }
                                },

                                // 3. everything else
                                _ => {
                                    for p in event.paths {
                                        debouncer.add_event(ProcessedEvent {
                                            path: p,
                                            old_path: None,
                                            new_path: None,
                                            kind: event.kind,
                                            priority: prio,
                                            timestamp: Instant::now(),
                                        });
                                    }
                                }
                            } // end match event.kind
                        } // <--- closes Ok(event)
                        Err(e) => eprintln!("watcher channel error: {:?}", e),
                    }

                    if processed_in_batch >= config_clone.batch_size {
                        break;
                    }
                }

                // deal with orphaned removes
                remove_tracker.flush_expired(Duration::from_millis(500), &mut debouncer);

                queue_size_clone.store(debouncer.len(), Ordering::SeqCst);

                // flush if ready
                if debouncer.is_ready_to_flush() && debouncer.len() > 0 {
                    let to_process = debouncer.flush();
                    events_processed_clone.fetch_add(to_process.len(), Ordering::SeqCst);

                    let maybe_db = db_for_thread.lock().ok().and_then(|g| g.clone());

                    for ev in &to_process {
                        if let Some(db_mutex) = &maybe_db {
                            // update DB for renames
                            if let EventKind::Modify(ModifyKind::Name(_)) = ev.kind {
                                if let (Some(old_p), Some(new_p)) = (&ev.old_path, &ev.new_path) {
                                    let old_s = to_db_path(old_p);
                                    let new_s = to_db_path(new_p);
                                    let res =
                                        handle_db_update(db_mutex, &old_s, &new_s, new_p.is_dir());
                                    if let Err(e) = res {
                                        eprintln!("DB rename error: {:?}", e);
                                    }
                                }
                            }
                            info!("processed (DB) {:?} {:?}", ev.kind, ev.path);
                        } else {
                            info!("processed       {:?} {:?}", ev.kind, ev.path);
                        }
                    }
                }

                thread::sleep(Duration::from_millis(50));
            } // main loop

            // final flush on shutdown
            remove_tracker.flush_expired(Duration::from_millis(500), &mut debouncer);
            if debouncer.len() > 0 {
                let final_evts = debouncer.flush();
                events_processed_clone.fetch_add(final_evts.len(), Ordering::SeqCst);
                for ev in &final_evts {
                    info!("processing final event {:?} {:?}", ev.kind, ev.path);
                }
            }

            if let Ok(mut g) = state_clone.lock() {
                *g = WatcherState::Stopped;
            }
        });

        // ── return constructed watcher ───────────────────────────────────────
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

    // ── public API ////////////////////////////////////////////////////////////
    pub fn with_database(&mut self, db: Arc<Mutex<Database>>) -> Result<&mut Self> {
        *self
            .db_shared
            .lock()
            .map_err(|_| anyhow::anyhow!("db mutex poisoned"))? = Some(db);
        Ok(self)
    }

    pub fn start(&mut self) -> Result<()> {
        let mut g = self.state.lock().map_err(|_| anyhow::anyhow!("state"))?;
        match *g {
            WatcherState::Initializing | WatcherState::Paused => {
                *g = WatcherState::Watching;
                Ok(())
            }
            WatcherState::Watching => Ok(()), // idempotent
            _ => Err(anyhow::anyhow!("cannot start from {:?}", *g)),
        }
    }

    pub fn pause(&mut self) -> Result<()> {
        let mut g = self.state.lock().map_err(|_| anyhow::anyhow!("state"))?;
        match *g {
            WatcherState::Watching => {
                *g = WatcherState::Paused;
                Ok(())
            }
            WatcherState::Paused => Ok(()),
            _ => Err(anyhow::anyhow!("cannot pause from {:?}", *g)),
        }
    }

    pub fn resume(&mut self) -> Result<()> {
        let mut g = self.state.lock().map_err(|_| anyhow::anyhow!("state"))?;
        match *g {
            WatcherState::Paused => {
                *g = WatcherState::Watching;
                Ok(())
            }
            WatcherState::Watching => Ok(()),
            _ => Err(anyhow::anyhow!("cannot resume from {:?}", *g)),
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        {
            let mut g = self.state.lock().map_err(|_| anyhow::anyhow!("state"))?;
            if matches!(*g, WatcherState::Stopped | WatcherState::ShuttingDown) {
                return Ok(());
            }
            *g = WatcherState::ShuttingDown;
        }

        self.stop_flag.store(true, Ordering::SeqCst);

        if let Some(h) = self.processor_thread.take() {
            let _ = h.join();
        }

        *self.state.lock().map_err(|_| anyhow::anyhow!("state"))? = WatcherState::Stopped;
        Ok(())
    }

    pub fn status(&self) -> Result<WatcherStatus> {
        let st = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state"))?
            .clone();
        Ok(WatcherStatus {
            state: st,
            events_processed: self.events_processed.load(Ordering::SeqCst),
            queue_size: self.queue_size.load(Ordering::SeqCst),
            start_time: Some(self.start_time),
            watched_paths: self.watched_paths.clone(),
        })
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        let _ = self.stop(); // ignore errors during drop
    }
}

// ────── tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod event_debouncer_tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode};
    use std::fs;
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
            old_path: None,
            new_path: None,
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
            old_path: None,
            new_path: None,
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: t1,
        });
        std::thread::sleep(Duration::from_millis(10));
        let t2 = Instant::now();
        debouncer.add_event(ProcessedEvent {
            path: path1.clone(),
            old_path: None,
            new_path: None,
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
            old_path: None,
            new_path: None,
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        assert_eq!(debouncer_h.len(), 1);

        debouncer_h.add_event(ProcessedEvent {
            path: p_dir.clone(),
            old_path: None,
            new_path: None,
            kind: EventKind::Remove(RemoveKind::Folder),
            priority: EventPriority::Delete,
            timestamp: Instant::now(),
        });
        assert_eq!(
            debouncer_h.len(),
            1,
            "Hierarchical debounce should remove child event, leaving only parent dir event"
        );

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
            old_path: None,
            new_path: None,
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        debouncer.add_event(ProcessedEvent {
            path: path2.clone(),
            old_path: None,
            new_path: None,
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

        debouncer.add_event(ProcessedEvent {
            path: path1,
            old_path: None,
            new_path: None,
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            priority: EventPriority::Modify,
            timestamp: Instant::now(),
        });
        debouncer.add_event(ProcessedEvent {
            path: path2,
            old_path: None,
            new_path: None,
            kind: EventKind::Create(CreateKind::File),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        debouncer.add_event(ProcessedEvent {
            path: path3,
            old_path: None,
            new_path: None,
            kind: EventKind::Remove(RemoveKind::File),
            priority: EventPriority::Delete,
            timestamp: Instant::now(),
        });

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
            old_path: None,
            new_path: None,
            kind: EventKind::Create(CreateKind::Folder),
            priority: EventPriority::Create,
            timestamp: Instant::now(),
        });
        debouncer.add_event(ProcessedEvent {
            path: file,
            old_path: None,
            new_path: None,
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

    #[test]
    fn remove_create_same_inode_produces_rename() {
        let tmp = tempfile::tempdir().unwrap();
        let old_p = tmp.path().join("old.txt");
        std::fs::write(&old_p, b"hi").unwrap();

        let mut debouncer = EventDebouncer::new(100);
        let mut tracker = RemoveTracker::default();

        tracker.record(&old_p);

        let new_p = tmp.path().join("new.txt");
        std::fs::rename(&old_p, &new_p).unwrap();

        if let Some(orig) = tracker.match_create(&new_p, Duration::from_millis(500)) {
            debouncer.add_event(ProcessedEvent {
                path: orig.clone(),
                old_path: Some(orig),
                new_path: Some(new_p.clone()),
                kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                priority: EventPriority::Modify,
                timestamp: Instant::now(),
            });
        }

        tracker.flush_expired(Duration::from_millis(500), &mut debouncer);
        let flushed = debouncer.flush();
        assert_eq!(flushed.len(), 1);
        assert_eq!(
            flushed[0].kind,
            EventKind::Modify(ModifyKind::Name(RenameMode::Both))
        );
        assert_eq!(
            flushed[0].old_path.as_ref().unwrap(),
            &tmp.path().join("old.txt")
        );
        assert_eq!(flushed[0].new_path.as_ref().unwrap(), &new_p);
    }
}

#[cfg(test)]
mod file_watcher_state_tests {
    use super::*;
    use std::fs as FsMod;
    use tempfile::tempdir;

    #[test]
    fn test_watcher_pause_resume_stop() {
        let tmp_dir = tempdir().unwrap();
        let watch_path = tmp_dir.path().to_path_buf();
        FsMod::create_dir_all(&watch_path).expect("Failed to create temp dir for watching");

        let config = WatcherConfig::default();
        let mut watcher =
            FileWatcher::new(vec![watch_path], config).expect("Failed to create watcher");

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
        let mut watcher =
            FileWatcher::new(vec![tmp_dir.path().to_path_buf()], WatcherConfig::default()).unwrap();

        // already watching
        {
            let mut g = watcher.state.lock().unwrap();
            *g = WatcherState::Watching;
        }
        assert!(watcher.start().is_ok());
        assert_eq!(watcher.status().unwrap().state, WatcherState::Watching);

        // invalid transition
        {
            let mut g = watcher.state.lock().unwrap();
            *g = WatcherState::ShuttingDown;
        }
        assert!(watcher.start().is_err());
    }

    #[test]
    fn test_new_watcher_with_nonexistent_path() {
        let bogus =
            PathBuf::from("/path/that/REALLY/does/not/exist/for/sure/and/cannot/be/created");
        let res = FileWatcher::new(vec![bogus], WatcherConfig::default());
        assert!(res.is_err());
        if let Err(e) = res {
            let msg = e.to_string();
            assert!(
                msg.contains("Failed to watch path") || msg.contains("os error 2"),
                "got: {msg}"
            );
        }
    }

    #[test]
    fn test_watcher_default_config() {
        let cfg = WatcherConfig::default();
        assert_eq!(cfg.debounce_ms, 100);
        assert_eq!(cfg.batch_size, 1_000);
        assert_eq!(cfg.max_queue_size, 100_000);
        assert_eq!(cfg.drain_timeout_ms, 5_000);
    }

    #[test]
    fn test_poisoned_state_mutex_errors() {
        let tmp_dir = tempdir().unwrap();
        let watch_path = tmp_dir.path().to_path_buf();
        FsMod::create_dir_all(&watch_path).unwrap();

        let mut watcher =
            FileWatcher::new(vec![watch_path], WatcherConfig::default()).expect("create");

        let state_arc = watcher.state.clone();
        let _ = std::thread::spawn(move || {
            let _g = state_arc.lock().unwrap();
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
