use anyhow::Result;
use clap::Subcommand;
use libmarlin::config::Config;
use libmarlin::watcher::{WatcherConfig, WatcherState, WatcherStatus};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;

#[allow(dead_code)]
static LAST_WATCHER_STATE: once_cell::sync::Lazy<Mutex<Option<WatcherState>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

#[allow(dead_code)]
pub fn last_watcher_state() -> Option<WatcherState> {
    LAST_WATCHER_STATE.lock().unwrap().clone()
}

#[derive(Subcommand, Debug)]
pub enum WatchCmd {
    Start {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "100")]
        debounce_ms: u64,
    },
    Status,
    Stop,
    #[command(hide = true)]
    Daemon {
        path: PathBuf,
        debounce_ms: u64,
        port: u16,
        control: PathBuf,
    },
}

#[derive(Serialize, Deserialize)]
struct ControlInfo {
    pid: u32,
    port: u16,
}

#[derive(Serialize, Deserialize)]
struct StatusDto {
    state: String,
    events_processed: usize,
    queue_size: usize,
    uptime_secs: u64,
}

fn control_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("watch.json")
}

fn choose_port(db_path: &Path) -> u16 {
    let mut h = DefaultHasher::new();
    db_path.hash(&mut h);
    31000 + ((h.finish() % 1000) as u16)
}

fn read_control(path: &Path) -> Result<ControlInfo> {
    let txt = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&txt)?)
}

fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;
        kill(Pid::from_raw(pid as i32), None).is_ok()
    }
    #[cfg(not(unix))]
    {
        true // fallback, assume alive
    }
}

fn send_request(port: u16, msg: &str) -> Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.write_all(msg.as_bytes())?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    Ok(buf)
}

fn status_to_dto(st: WatcherStatus) -> StatusDto {
    StatusDto {
        state: format!("{:?}", st.state),
        events_processed: st.events_processed,
        queue_size: st.queue_size,
        uptime_secs: st
            .start_time
            .map(|t| t.elapsed().as_secs())
            .unwrap_or_default(),
    }
}

pub fn run(cmd: &WatchCmd, _conn: &mut Connection, fmt: super::Format) -> Result<()> {
    match cmd {
        WatchCmd::Start { path, debounce_ms } => {
            let cfg = Config::load()?;
            let control = control_path(&cfg.db_path);
            if control.exists() {
                let info = read_control(&control)?;
                if process_alive(info.pid) {
                    info!("Watcher already running with PID {}", info.pid);
                    return Ok(());
                } else {
                    std::fs::remove_file(&control).ok();
                }
            }
            let port = choose_port(&cfg.db_path);
            let exe = std::env::current_exe()?;
            let child = std::process::Command::new(exe)
                .arg("watch")
                .arg("daemon")
                .arg("--path")
                .arg(path)
                .arg("--debounce-ms")
                .arg(debounce_ms.to_string())
                .arg("--port")
                .arg(port.to_string())
                .arg("--control")
                .arg(&control)
                .spawn()?;
            info!("Started watcher daemon with PID {}", child.id());
            Ok(())
        }
        WatchCmd::Daemon {
            path,
            debounce_ms,
            port,
            control,
        } => {
            let mut marlin = libmarlin::Marlin::open_default()?;
            let config = WatcherConfig {
                debounce_ms: *debounce_ms,
                ..Default::default()
            };
            let canon_path = path.canonicalize().unwrap_or_else(|_| path.clone());
            let watcher = Arc::new(Mutex::new(marlin.watch(&canon_path, Some(config))?));
            let running = Arc::new(AtomicBool::new(true));
            let srv_running = running.clone();
            let w_clone = watcher.clone();
            let port_val = *port;
            let server = thread::spawn(move || {
                let listener = TcpListener::bind(("127.0.0.1", port_val)).unwrap();
                for mut s in listener.incoming().flatten() {
                    let mut buf = String::new();
                    if s.read_to_string(&mut buf).is_ok() {
                        if buf.contains("status") {
                            if let Ok(st) = w_clone.lock().unwrap().status() {
                                let dto = status_to_dto(st);
                                let _ =
                                    s.write_all(serde_json::to_string(&dto).unwrap().as_bytes());
                            }
                        } else if buf.contains("stop") {
                            let _ = s.write_all(b"ok");
                            srv_running.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                }
            });
            let info = ControlInfo {
                pid: std::process::id(),
                port: *port,
            };
            std::fs::write(control, serde_json::to_string(&info)?)?;
            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(200));
            }
            watcher.lock().unwrap().stop()?;
            server.join().ok();
            std::fs::remove_file(control).ok();
            {
                let mut guard = LAST_WATCHER_STATE.lock().unwrap();
                *guard = Some(WatcherState::Stopped);
            }
            Ok(())
        }
        WatchCmd::Status => {
            let cfg = Config::load()?;
            let control = control_path(&cfg.db_path);
            if !control.exists() {
                info!("Status command: No active watcher process to query …");
                return Ok(());
            }
            let info = read_control(&control)?;
            let resp = send_request(info.port, "status");
            match resp {
                Ok(txt) => {
                    if fmt == super::Format::Json {
                        println!("{txt}");
                    } else {
                        let dto: StatusDto = serde_json::from_str(&txt)?;
                        println!(
                            "state: {} processed:{} queue:{} uptime:{}s",
                            dto.state, dto.events_processed, dto.queue_size, dto.uptime_secs
                        );
                    }
                }
                Err(_) => {
                    info!("Failed to query watcher status");
                }
            }
            Ok(())
        }
        WatchCmd::Stop => {
            let cfg = Config::load()?;
            let control = control_path(&cfg.db_path);
            if !control.exists() {
                info!("Stop command: No active watcher process to stop …");
                return Ok(());
            }
            let info = read_control(&control)?;
            let _ = send_request(info.port, "stop");
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(5) {
                if !process_alive(info.pid) {
                    break;
                }
                thread::sleep(Duration::from_millis(200));
            }
            if process_alive(info.pid) {
                #[cfg(unix)]
                {
                    use nix::sys::signal::{kill, Signal};
                    use nix::unistd::Pid;
                    let _ = kill(Pid::from_raw(info.pid as i32), Signal::SIGTERM);
                }
            }
            std::fs::remove_file(control).ok();
            Ok(())
        }
    }
}
