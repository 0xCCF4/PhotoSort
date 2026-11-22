use crate::build_file_patterns;
use crate::testutils::{
    analyzer, find, nested_tree, path_pattern, pattern_separator, patterns, relative, TempDir,
    DEEP_PHOTO, ROOT_PHOTO, SUB_PHOTO,
};

#[test]
fn recursive_scan_finds_nested_files() {
    let dir = nested_tree("recursive_all");
    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![
            ROOT_PHOTO.to_string(),
            format!("sub/{SUB_PHOTO}"),
            format!("sub/nested/{DEEP_PHOTO}"),
        ]
    );
}

#[test]
fn non_recursive_scan_finds_root_file_only() {
    let dir = nested_tree("non_recursive");
    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());

    assert_eq!(
        relative(&find(&analyzer, false), dir.path()),
        vec![ROOT_PHOTO.to_string()]
    );
}

/// A wildcard pattern with a directory part is matched relative to the source
/// root. This is the regression test for the path being stripped of the file's
/// own parent directory instead of the source root.
#[test]
fn exclude_directory_relative_to_source_root() {
    let dir = nested_tree("exclude_dir");
    let excluded = patterns(&[&path_pattern(&["sub", "**"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![ROOT_PHOTO.to_string()]
    );
}

#[test]
fn exclude_nested_directory_relative_to_source_root() {
    let dir = nested_tree("exclude_nested_dir");
    let excluded = patterns(&[&path_pattern(&["sub", "nested", "**"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![ROOT_PHOTO.to_string(), format!("sub/{SUB_PHOTO}")]
    );
}

#[test]
fn exclude_directory_relative_to_source_root_with_single_star() {
    let dir = nested_tree("exclude_dir_star");
    let excluded = patterns(&[&path_pattern(&["sub", "*"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);

    // `*` does not cross a path separator, so the nested file stays.
    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![ROOT_PHOTO.to_string(), format!("sub/nested/{DEEP_PHOTO}")]
    );
}

#[test]
fn exclude_directory_with_regex() {
    let dir = nested_tree("exclude_regex");
    let excluded = build_file_patterns(
        [] as [&str; 0],
        [format!("^{}", path_pattern(&["sub"]))],
        false,
    )
    .expect("Invalid test pattern");
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![ROOT_PHOTO.to_string()]
    );
}

/// A pattern without a directory part matches in the source root only.
/// `**/Thumbs.db` matches that name at every level.
#[test]
fn exclude_file_name_in_source_root_only() {
    let dir = TempDir::new("exclude_root_name");
    dir.create_file("Thumbs.db");
    dir.create_file("sub/Thumbs.db");
    dir.create_file("sub/nested/Thumbs.db");
    let excluded = patterns(&[&path_pattern(&["Thumbs.db"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![
            "sub/Thumbs.db".to_string(),
            "sub/nested/Thumbs.db".to_string()
        ]
    );
}

#[test]
fn exclude_file_name_at_all_levels() {
    let dir = TempDir::new("exclude_all_levels");
    dir.create_file("Thumbs.db");
    dir.create_file("sub/Thumbs.db");
    dir.create_file("sub/nested/Thumbs.db");
    dir.create_file("keep.jpg");
    let excluded = patterns(&[&format!("**{}{}", pattern_separator(), "Thumbs.db")]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec!["keep.jpg".to_string()]
    );
}

#[test]
fn exclude_documented_example() {
    let dir = TempDir::new("exclude_documented");
    dir.create_file("Thumbs.db");
    dir.create_file("sub/Thumbs.db");
    dir.create_file("abc/x/test/Thumbs.db");
    dir.create_file("abc/x/test/keep.jpg");
    let excluded = patterns(&[&path_pattern(&["abc", "*", "test", "**", "Thumbs.db"])]);
    let analyzer = analyzer(dir.path(), Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![
            "Thumbs.db".to_string(),
            "abc/x/test/keep.jpg".to_string(),
            "sub/Thumbs.db".to_string(),
        ]
    );
}

#[test]
fn include_direct_children_only() {
    let dir = nested_tree("include_children");
    let included = patterns(&[&path_pattern(&["sub", "*"])]);
    let analyzer = analyzer(dir.path(), included, Vec::new());

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![format!("sub/{SUB_PHOTO}")]
    );
}

#[test]
fn include_nested_paths() {
    let dir = nested_tree("include_nested");
    let included = patterns(&[&path_pattern(&["sub", "**"])]);
    let analyzer = analyzer(dir.path(), included, Vec::new());

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![
            format!("sub/{SUB_PHOTO}"),
            format!("sub/nested/{DEEP_PHOTO}")
        ]
    );
}

#[test]
fn exclude_takes_priority_over_include() {
    let dir = nested_tree("include_exclude");
    let included = patterns(&[&path_pattern(&["sub", "**"])]);
    let excluded = patterns(&[&path_pattern(&["sub", "nested", "**"])]);
    let analyzer = analyzer(dir.path(), included, excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![format!("sub/{SUB_PHOTO}")]
    );
}

#[test]
fn pattern_ignores_case_if_configured() {
    let case_sensitive =
        build_file_patterns([path_pattern(&["thumbs.db"])], [] as [&str; 0], false)
            .expect("Invalid test pattern");
    assert!(!case_sensitive[0].is_match(&path_pattern(&["Thumbs.db"])));

    let case_insensitive =
        build_file_patterns([path_pattern(&["thumbs.db"])], [] as [&str; 0], true)
            .expect("Invalid test pattern");
    assert!(case_insensitive[0].is_match(&path_pattern(&["Thumbs.db"])));
}

#[test]
fn invalid_wildcard_pattern_is_rejected() {
    let separator = pattern_separator();
    let result = build_file_patterns(
        [format!("/{separator}**{separator}**{separator}")],
        [] as [&str; 0],
        false,
    );

    assert!(result.is_err());
}

/// The source directory may contain `.` and `..` parts. Paths are still
/// matched relative to the resolved source directory.
#[test]
fn source_directory_with_dot_parts() {
    let dir = nested_tree("source_dot_parts");
    let source = dir.path().join("sub").join("..");
    let excluded = patterns(&[&path_pattern(&["sub", "**"])]);
    let analyzer = analyzer(&source, Vec::new(), excluded);

    assert_eq!(
        relative(&find(&analyzer, true), dir.path()),
        vec![ROOT_PHOTO.to_string()]
    );
}

/// A file is added to the result once, also when the scan runs twice.
#[test]
fn repeated_scan_adds_files_once() {
    let dir = nested_tree("repeated_scan");
    let analyzer = analyzer(dir.path(), Vec::new(), Vec::new());
    let mut files = Vec::new();

    for _ in 0..2 {
        analyzer
            .find_files_in_source(dir.path(), true, &mut files)
            .expect("Failed to scan source directory");
    }

    assert_eq!(files.len(), 3);
}
