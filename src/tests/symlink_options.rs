//! Tests for the symbolic link options of the scan and for its robustness.
//!
//! These tests cover `dont_follow_symlinked_dirs`, `dont_follow_symlinked_files`,
//! the deduplication of files that are reached more than once, link cycles and
//! dangling links.

#![cfg(unix)]

use crate::testutils::{
    analyzer, analyzer_with, find, AnalyzerOptions, TempDir, ROOT_PHOTO, SUB_PHOTO,
};

/// A linked directory that is also a source directory is scanned once.
#[test]
fn symlinked_dir_is_searched_only_once() {
    let dir = TempDir::new("symlink_sources_once");
    dir.create_file(&format!("real/{ROOT_PHOTO}"));
    let alias = dir.symlink_dir(&dir.path().join("real"), "alias");

    let analyzer = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            source_dirs: vec![dir.path().join("real"), alias],
            ..Default::default()
        },
    );
    let files = find(&analyzer, true);

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with(ROOT_PHOTO));
}

/// With `dont_follow_symlinked_dirs`, a linked directory is not scanned.
#[test]
fn dont_follow_symlinked_dirs_skips_linked_dir() {
    let dir = TempDir::new("dont_follow_dir");
    dir.create_file(&format!("scan/{ROOT_PHOTO}"));
    dir.create_file(&format!("external/{SUB_PHOTO}"));
    dir.symlink_dir(&dir.path().join("external"), "scan/link");
    let scan = dir.path().join("scan");

    let following = analyzer(&scan, Vec::new(), Vec::new());
    assert_eq!(find(&following, true).len(), 2);

    let not_following = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            source_dirs: vec![scan],
            dont_follow_symlinked_dirs: true,
            ..Default::default()
        },
    );
    let files = find(&not_following, true);
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with(ROOT_PHOTO));
}

/// With `dont_follow_symlinked_files`, a linked file is not collected.
#[test]
fn dont_follow_symlinked_files_skips_linked_file() {
    let dir = TempDir::new("dont_follow_file");
    dir.create_file(&format!("scan/{ROOT_PHOTO}"));
    let target = dir.create_file(&format!("external/{SUB_PHOTO}"));
    dir.symlink_file(&target, "scan/link.jpg");
    let scan = dir.path().join("scan");

    let following = analyzer(&scan, Vec::new(), Vec::new());
    assert_eq!(find(&following, true).len(), 2);

    let not_following = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            source_dirs: vec![scan],
            dont_follow_symlinked_files: true,
            ..Default::default()
        },
    );
    let files = find(&not_following, true);
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with(ROOT_PHOTO));
}

/// The two symlink options work independently.
#[test]
fn file_and_dir_flags_are_independent() {
    let dir = TempDir::new("symlink_flags_independent");
    dir.create_file(&format!("scan/{ROOT_PHOTO}"));
    let target = dir.create_file(&format!("external/{SUB_PHOTO}"));
    dir.symlink_file(&target, "scan/file_link.jpg");
    dir.symlink_dir(&dir.path().join("external"), "scan/dir_link");
    let scan = dir.path().join("scan");

    // Skipping dir links keeps the file link, which points to the same file as
    // the dir link.
    let dirs_skipped = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            source_dirs: vec![scan.clone()],
            dont_follow_symlinked_dirs: true,
            ..Default::default()
        },
    );
    assert_eq!(find(&dirs_skipped, true).len(), 2);

    // Skipping file links keeps the dir link, so the other file is still reached.
    let files_skipped = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            source_dirs: vec![scan],
            dont_follow_symlinked_files: true,
            ..Default::default()
        },
    );
    assert_eq!(find(&files_skipped, true).len(), 2);
}

/// A linked file is listed once for every combination of the symlink options.
#[test]
fn symlinked_file_is_listed_only_once() {
    let dir = TempDir::new("symlink_file_dedupe");
    let target = dir.create_file(&format!("scan/{ROOT_PHOTO}"));
    dir.symlink_file(&target, "scan/link.jpg");
    let scan = dir.path().join("scan");

    for (dont_follow_symlinked_dirs, dont_follow_symlinked_files) in
        [(false, false), (false, true), (true, false), (true, true)]
    {
        let analyzer = analyzer_with(
            dir.path(),
            AnalyzerOptions {
                source_dirs: vec![scan.clone()],
                dont_follow_symlinked_dirs,
                dont_follow_symlinked_files,
                ..Default::default()
            },
        );
        let files = find(&analyzer, true);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with(ROOT_PHOTO));
    }
}

/// A link back to a parent directory does not recurse endlessly.
#[test]
fn symlink_cycle_does_not_recurse_endlessly() {
    let dir = TempDir::new("symlink_cycle");
    dir.create_file(&format!("a/b/{ROOT_PHOTO}"));
    dir.symlink_dir(dir.path(), "a/b/back");

    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());
    let files = find(&analyzer, true);

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with(ROOT_PHOTO));
}

/// A dangling link is skipped and does not stop the scan.
#[test]
fn broken_symlink_does_not_stop_the_scan() {
    let dir = TempDir::new("symlink_broken");
    dir.create_file(&format!("scan/{ROOT_PHOTO}"));
    dir.symlink_file(
        std::path::Path::new("/nonexistent/photo.jpg"),
        "scan/broken.jpg",
    );

    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());
    let files = find(&analyzer, true);

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with(ROOT_PHOTO));
}
