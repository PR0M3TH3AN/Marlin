use tracing_subscriber::{fmt, EnvFilter};

/// Initialise global tracing subscriber.
///
/// Reads `RUST_LOG` for filtering, falls back to `info`.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter(filter)
        .init();
}
