use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    /// Global mutex to serialize environment-variable modifications in tests.
    pub static ref ENV_MUTEX: Mutex<()> = Mutex::new(());
}
