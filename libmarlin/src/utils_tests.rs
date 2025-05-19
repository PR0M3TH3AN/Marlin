// libmarlin/src/utils_tests.rs

use super::utils::determine_scan_root;
use std::path::PathBuf;

#[test]
fn determine_scan_root_plain_path() {
    let root = determine_scan_root("foo/bar/baz.txt");
    assert_eq!(root, PathBuf::from("foo/bar"));
}

#[test]
fn determine_scan_root_glob() {
    let root = determine_scan_root("foo/*/baz.rs");
    assert_eq!(root, PathBuf::from("foo"));
}

#[test]
fn determine_scan_root_only_wildcards() {
    let root = determine_scan_root("**/*.txt");
    assert_eq!(root, PathBuf::from("."));
}
