// libmarlin/src/logging_tests.rs

use super::logging;
use tracing::Level;

#[test]
fn init_sets_up_subscriber() {
    // set RUST_LOG to something to test the EnvFilter path
    std::env::set_var("RUST_LOG", "debug");
    logging::init();
    tracing::event!(Level::INFO, "this is a test log");
    // if we made it here without panic, we’re good
}
