//! Misc shared helpers.

use std::path::PathBuf;

/// Determine a filesystem root to limit recursive walking on glob scans.
///
/// If the pattern contains any of `*?[`, we take everything up to the
/// first such character, and then (if that still contains metacharacters)
/// walk up until there aren’t any left.  If there are *no* metachars at
/// all, we treat the entire string as a path and return its parent
/// directory (or `.` if it has no parent).
pub fn determine_scan_root(pattern: &str) -> PathBuf {
    // find first wildcard char
    let first_wild = pattern
        .find(|c| matches!(c, '*' | '?' | '['))
        .unwrap_or(pattern.len());

    // everything up to the wildcard (or the whole string if none)
    let prefix = &pattern[..first_wild];
    let mut root = PathBuf::from(prefix);

    // If there were NO wildcards at all, just return the parent directory
    if first_wild == pattern.len() {
        return root
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
    }

    // Otherwise, if the prefix still has any wildcards (e.g. "foo*/bar"),
    // walk back up until it doesn’t
    while root
        .as_os_str()
        .to_string_lossy()
        .chars()
        .any(|c| matches!(c, '*' | '?' | '['))
    {
        root = root.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    }

    if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root
    }
}
