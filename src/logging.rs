use tracing_subscriber::{fmt, EnvFilter};

/// Initialise global tracing subscriber.
///
/// Reads `RUST_LOG` for filtering, falls back to `info`.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // All tracing output (INFO, WARN, ERROR …) now goes to *stderr* so the
    // integration tests can assert on warnings / errors reliably.
    fmt()
        .with_target(false)        // hide module targets
        .with_level(true)          // include log level
        .with_env_filter(filter)   // respect RUST_LOG
        .with_writer(std::io::stderr) // <-- NEW: send to stderr
        .init();
}
