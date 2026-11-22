//! Tests for the handling of symbolic links during the source scan.
//!
//! The scan follows links and matches a file with the path under which it
//! appears in the source directory. The path in the result is the resolved
//! path of the file.

#![cfg(unix)]

use crate::testutils::{
    analyzer, find, nested_tree, path_pattern, patterns, relative, TempDir, ROOT_PHOTO, SUB_PHOTO,
};

/// A linked directory inside the source tree is scanned and its files are
/// matched with the path through the link.
#[test]
fn symlink_to_directory_inside_source_matches_visible_path() {
    let dir = nested_tree("symlink_dir_inside");
    dir.symlink_dir(&dir.path().join("sub"), "link");

    // The files below `sub` are found directly and through the link. The
    // resolved path is the same, so each file appears once.
    let unfiltered = analyzer(dir.path(), Vec::new(), Vec::new());
    assert_eq!(find(&unfiltered, true).len(), 3);

    // A pattern for the link excludes the files behind the link only.
    let excluded = patterns(&[&path_pattern(&["link", "**"])]);
    let excluded_analyzer = analyzer(dir.path(), Vec::new(), excluded);
    assert_eq!(
        relative(&find(&excluded_analyzer, true), dir.path()),
        vec![
            ROOT_PHOTO.to_string(),
            format!("sub/{SUB_PHOTO}"),
            format!("sub/nested/{}", crate::testutils::DEEP_PHOTO),
        ]
    );

    // An include pattern for the link keeps the files behind the link only.
    // The exclude pattern for the target directory does not remove them,
    // because the scan matches the path through the link.
    let included = patterns(&[&path_pattern(&["link", "**"])]);
    let excluded = patterns(&[&path_pattern(&["sub", "**"])]);
    let included_analyzer = analyzer(dir.path(), included, excluded);
    assert_eq!(find(&included_analyzer, true).len(), 2);
}

/// A link to a directory outside the source tree is followed and matched with
/// the path through the link.
#[test]
fn symlink_to_directory_outside_source_matches_visible_path() {
    let dir = nested_tree("symlink_dir_outside");
    let outside = TempDir::new("symlink_dir_outside_target");
    let outside_file = outside.create_file("outside_20230102_101530.jpg");
    dir.symlink_dir(outside.path(), "link");

    let unfiltered = analyzer(dir.path(), Vec::new(), Vec::new());
    let files = find(&unfiltered, true);
    assert_eq!(files.len(), 4);
    assert!(files.contains(&outside_file.canonicalize().expect("Invalid path")));

    let excluded = patterns(&[&path_pattern(&["link", "**"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);
    assert_eq!(find(&analyzer, true).len(), 3);
}

/// A linked file inside the source tree is matched with the name of the link.
#[test]
fn symlink_to_file_inside_source_matches_visible_path() {
    let dir = nested_tree("symlink_file_inside");
    dir.symlink_file(&dir.path().join(format!("sub/{SUB_PHOTO}")), "link.jpg");

    // The pattern for `sub` removes the direct entries only, the link stays.
    let excluded = patterns(&[&path_pattern(&["sub", "**"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);
    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![ROOT_PHOTO.to_string(), format!("sub/{SUB_PHOTO}")]
    );
}

/// A linked file inside the source tree is matched with the name of the link
/// and processed once.
#[test]
fn symlink_to_file_inside_source_appears_once() {
    let dir = nested_tree("symlink_file_once");
    let target = dir.path().join(format!("sub/{SUB_PHOTO}"));
    dir.symlink_file(&target, "link.jpg");

    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());
    let files = find(&analyzer, true);
    let target = target.canonicalize().expect("Invalid path");

    assert_eq!(files.len(), 3);
    assert_eq!(files.iter().filter(|path| **path == target).count(), 1);
}

/// Two links to the same directory do not add the files below it twice.
#[test]
fn two_symlinks_to_the_same_directory_add_files_once() {
    let dir = nested_tree("symlink_two_links");
    dir.symlink_dir(&dir.path().join("sub"), "link1");
    dir.symlink_dir(&dir.path().join("sub"), "link2");

    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());
    let files = find(&analyzer, true);

    assert_eq!(files.len(), 3);
}

/// A linked file outside the source tree is matched with the name of the link.
#[test]
fn symlink_to_file_outside_source_matches_visible_path() {
    let dir = nested_tree("symlink_file_outside");
    let outside = TempDir::new("symlink_file_outside_target");
    let outside_file = outside.create_file("outside_20230102_101530.jpg");
    dir.symlink_file(&outside_file, "link.jpg");

    let unfiltered = analyzer(dir.path(), Vec::new(), Vec::new());
    let files = find(&unfiltered, true);
    assert_eq!(files.len(), 4);
    assert!(files.contains(&outside_file.canonicalize().expect("Invalid path")));

    let excluded = patterns(&[&path_pattern(&["link.jpg"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);
    assert_eq!(find(&analyzer, true).len(), 3);
}

/// A linked file outside the source tree can be selected by a pattern for the
/// name of the link.
#[test]
fn symlink_to_file_outside_source_can_be_included() {
    let dir = nested_tree("symlink_file_outside_include");
    let outside = TempDir::new("symlink_file_outside_include_target");
    let outside_file = outside.create_file("outside_20230102_101530.jpg");
    dir.symlink_file(&outside_file, "link.jpg");

    let included = patterns(&[&path_pattern(&["*.jpg"])]);
    let analyzer = analyzer(dir.path(), included, Vec::new());
    let files = find(&analyzer, true);

    assert_eq!(files.len(), 2);
    assert!(files.contains(&outside_file.canonicalize().expect("Invalid path")));
}

/// A link may be used as the source directory.
#[test]
fn symlink_source_directory() {
    let tree = nested_tree("symlink_source_root");
    let link_dir = TempDir::new("symlink_source_root_link");
    let source = link_dir.symlink_dir(tree.path(), "source");

    let excluded = patterns(&[&path_pattern(&["sub", "**"])]);
    let analyzer = analyzer(&source, Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), tree.path()),
        vec![ROOT_PHOTO.to_string()]
    );
}

/// A link loop terminates with an error instead of an endless scan.
#[test]
fn symlink_loop_returns_error() {
    let dir = TempDir::new("symlink_loop");
    dir.create_file(ROOT_PHOTO);
    dir.symlink_dir(dir.path(), "loop");

    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());
    let mut files = Vec::new();
    let result = analyzer.find_files_in_source(dir.path(), true, &mut files);

    assert!(result.is_err());
}

/// A dangling link cannot be resolved and aborts the scan of the source
/// directory. Files found before the error stay in the result.
#[test]
fn dangling_symlink_returns_error() {
    let dir = TempDir::new("symlink_dangling");
    dir.create_file(ROOT_PHOTO);
    dir.symlink_file(std::path::Path::new("/nonexistent/photo.jpg"), "broken.jpg");

    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());
    let mut files = Vec::new();
    let result = analyzer.find_files_in_source(dir.path(), true, &mut files);

    assert!(result.is_err());
}
