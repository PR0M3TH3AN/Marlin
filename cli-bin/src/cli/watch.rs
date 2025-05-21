// src/cli/watch.rs

use anyhow::Result;
use clap::Subcommand;
use libmarlin::watcher::{WatcherConfig, WatcherState};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;

use once_cell::sync::Lazy;
use std::sync::Mutex;

#[allow(dead_code)]
static LAST_WATCHER_STATE: Lazy<Mutex<Option<WatcherState>>> = Lazy::new(|| Mutex::new(None));

#[allow(dead_code)]
pub fn last_watcher_state() -> Option<WatcherState> {
    LAST_WATCHER_STATE.lock().unwrap().clone()
}

/// Commands related to file watching functionality
#[derive(Subcommand, Debug)]
pub enum WatchCmd {
    /// Start watching a directory for changes
    Start {
        /// Directory to watch (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        
        /// Debounce window in milliseconds (default: 100ms)
        #[arg(long, default_value = "100")]
        debounce_ms: u64,
    },
    
    /// Show status of currently active watcher
    Status,
    
    /// Stop the currently running watcher
    Stop,
}

/// Run a watch command
pub fn run(cmd: &WatchCmd, _conn: &mut Connection, _format: super::Format) -> Result<()> {
    match cmd {
        WatchCmd::Start { path, debounce_ms } => {
            let mut marlin = libmarlin::Marlin::open_default()?;
            let config = WatcherConfig {
                debounce_ms: *debounce_ms,
                ..Default::default()
            };
            let canon_path = path.canonicalize().unwrap_or_else(|_| path.clone());
            info!("Starting watcher for directory: {}", canon_path.display());

            let mut watcher = marlin.watch(&canon_path, Some(config))?;

            let status = watcher.status()?;
            info!("Watcher started. Press Ctrl+C to stop watching.");
            info!("Watching {} paths", status.watched_paths.len());
            
            let start_time = Instant::now();
            let mut last_status_time = Instant::now();
            let running = Arc::new(AtomicBool::new(true));
            let r_clone = running.clone();

            ctrlc::set_handler(move || {
                info!("Ctrl+C received. Signaling watcher to stop...");
                r_clone.store(false, Ordering::SeqCst);
            })?;

            info!("Watcher run loop started. Waiting for Ctrl+C or stop signal...");
            while running.load(Ordering::SeqCst) {
                let current_status = watcher.status()?;
                if current_status.state == WatcherState::Stopped {
                    info!("Watcher has stopped (detected by state). Exiting loop.");
                    break;
                }

                // Corrected line: removed the extra closing parenthesis
                if last_status_time.elapsed() > Duration::from_secs(10) { 
                    let uptime = start_time.elapsed();
                    info!(
                        "Watcher running for {}s, processed {} events, queue: {}, state: {:?}",
                        uptime.as_secs(),
                        current_status.events_processed,
                        current_status.queue_size,
                        current_status.state
                    );
                    last_status_time = Instant::now();
                }
                thread::sleep(Duration::from_millis(200));
            }

            info!("Watcher run loop ended. Explicitly stopping watcher instance...");
            watcher.stop()?;
            {
                let mut guard = LAST_WATCHER_STATE.lock().unwrap();
                *guard = Some(watcher.status()?.state);
            }
            info!("Watcher instance fully stopped.");
            Ok(())
        }
        WatchCmd::Status => {
            info!("Status command: No active watcher process to query in this CLI invocation model.");
            info!("To see live status, run 'marlin watch start' which prints periodic updates.");
            Ok(())
        }
        WatchCmd::Stop => {
            info!("Stop command: No active watcher process to stop in this CLI invocation model.");
            info!("Please use Ctrl+C in the terminal where 'marlin watch start' is running.");
            Ok(())
        }
    }
}