//! Tests for the interaction of `--fast` with the bracketing mode.

use crate::testutils::{analyzer_with, AnalyzerOptions, TempDir};
use std::path::PathBuf;

fn scanned(dir: &TempDir, fast: bool, bracketed: bool) -> Vec<String> {
    let analyzer = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            fast,
            bracketed_file_format: bracketed.then(|| "{bracket?group}_{name}.{ext}".to_string()),
            ..Default::default()
        },
    );

    let mut files = Vec::new();
    let mut visited = std::collections::HashSet::new();
    analyzer
        .find_files_in_source(dir.path(), false, &mut visited, &mut files)
        .expect("scan directory");

    files
        .into_iter()
        .map(|path: PathBuf| path.display().to_string())
        .collect()
}

fn tree(name: &str) -> TempDir {
    let dir = TempDir::new(name);
    // Creation order is not sorted order.
    for file in ["c.jpg", "a.jpg", "b.jpg"] {
        dir.create_file(file);
    }
    dir
}

#[test]
fn bracketing_keeps_the_sorted_order_with_fast() {
    let dir = tree("fast_bracket");
    let files = scanned(&dir, true, true);

    assert!(files.windows(2).all(|pair| pair[0] < pair[1]), "{files:?}");
}

#[test]
fn fast_without_bracketing_may_skip_the_sort() {
    let dir = tree("fast_plain");
    let analyzer = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            fast: true,
            ..Default::default()
        },
    );

    assert!(!analyzer.settings.sorted_order());
}

#[test]
fn bracketing_turns_the_sorted_order_on() {
    let dir = tree("fast_bracket_flag");
    let analyzer = analyzer_with(
        dir.path(),
        AnalyzerOptions {
            fast: true,
            bracketed_file_format: Some("{name}.{ext}".to_string()),
            ..Default::default()
        },
    );

    assert!(analyzer.settings.sorted_order());
}
