//! Misc shared helpers.

use std::path::PathBuf;

/// Determine a filesystem root to limit recursive walking on glob scans.
pub fn determine_scan_root(pattern: &str) -> PathBuf {
    let first_wild = pattern
        .find(|c| matches!(c, '*' | '?' | '['))
        .unwrap_or(pattern.len());
    let mut root = PathBuf::from(&pattern[..first_wild]);

    while root
        .as_os_str()
        .to_string_lossy()
        .contains(|c| matches!(c, '*' | '?' | '['))
    {
        root = root.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    }

    if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root
    }
}
