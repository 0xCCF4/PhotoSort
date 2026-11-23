//! A dry run over the test photos: every photo gets a plan line and nothing is
//! written to the target directory.
//!
//! The photos in `tests/src` are the same ones that the other tests read. They
//! are not part of the repository, so this test needs them in place.
#![cfg(feature = "binary")]

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Returns the directory that holds the test photos.
fn test_data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/src")
}

/// A temporary directory that is removed when the value is dropped.
struct TempDir {
    root: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> TempDir {
        let root =
            std::env::temp_dir().join(format!("photo_sort_dry_run_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        TempDir { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Runs the tool in dry-run mode over the test photos and returns its output.
///
/// The time zones are fixed, so the plan does not depend on the time zone of the
/// machine that runs the test.
fn dry_run(target: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_photo_sort"))
        .arg("--source-dir")
        .arg(test_data())
        .arg("--target-dir")
        .arg(target)
        .arg("--move-mode")
        .arg("copy")
        .arg("--analysis-mode")
        .arg("metadata_then_name")
        .arg("--extensions")
        .arg("jpg")
        .arg("--time-zone")
        .arg("utc")
        .arg("--output-time-zone")
        .arg("utc")
        .arg("--dry-run")
        .args(extra)
        .output()
        .expect("run photo_sort")
}

/// Returns the plan of a dry run as sorted `<photo> -> <target>` lines, with the
/// test data directory and the target directory removed.
fn plan(output: &Output, target: &Path) -> Vec<String> {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8");

    let mut lines: Vec<String> = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("[Copy] "))
        .filter(|line| {
            // A build with the video feature also processes the videos in the test
            // data. This test only checks the photos.
            line.split_once(" -> ").is_some_and(|(source, _)| {
                Path::new(source)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg"))
            })
        })
        .map(|line| {
            let (source, destination) = line.split_once(" -> ").expect("plan line with an arrow");
            let source = Path::new(source)
                .strip_prefix(test_data())
                .expect("source inside the test data");
            let destination = Path::new(destination)
                .strip_prefix(target)
                .expect("target inside the target directory");
            format!("{} -> {}", source.display(), destination.display())
        })
        .collect();
    lines.sort();
    lines
}

/// Returns the number of photos in the test data.
fn photo_count() -> usize {
    let Ok(entries) = fs::read_dir(test_data()) else {
        return 0;
    };

    entries
        .filter_map(std::result::Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg"))
        })
        .count()
}

/// Asserts that a run succeeded and that it wrote nothing.
fn assert_success_without_writes(output: &Output, target: &Path) {
    assert!(
        output.status.success(),
        "the dry run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_dir(target).expect("read target dir").count(),
        0,
        "the dry run wrote into the target directory"
    );
}

#[test]
#[ignore = "needs the photos in tests/src"]
fn dry_run_plans_every_photo() {
    let photos = photo_count();
    if photos == 0 {
        eprintln!("skipping: tests/src is missing the test photos");
        return;
    }

    let target = TempDir::new("plain");
    let output = dry_run(target.path(), &[]);
    assert_success_without_writes(&output, target.path());

    let plan = plan(&output, target.path());
    assert_eq!(plan.len(), photos, "plan:\n{}", plan.join("\n"));

    let sources: HashSet<&str> = plan
        .iter()
        .map(|line| line.split_once(" -> ").expect("arrow").0)
        .collect();
    assert_eq!(sources.len(), photos, "a photo was planned twice");

    let destinations: HashSet<&str> = plan
        .iter()
        .map(|line| line.split_once(" -> ").expect("arrow").1)
        .collect();
    assert_eq!(destinations.len(), photos, "two photos share a target");
}

#[test]
#[ignore = "needs the photos in tests/src"]
fn the_threaded_dry_run_prints_the_same_plan() {
    if photo_count() == 0 {
        eprintln!("skipping: tests/src is missing the test photos");
        return;
    }

    let plain_dir = TempDir::new("plain_plan");
    let threaded_dir = TempDir::new("threaded_plan");

    let plain_output = dry_run(plain_dir.path(), &[]);
    assert_success_without_writes(&plain_output, plain_dir.path());

    let threaded_output = dry_run(threaded_dir.path(), &["--threads", "4"]);
    assert_success_without_writes(&threaded_output, threaded_dir.path());

    assert_eq!(
        plan(&plain_output, plain_dir.path()),
        plan(&threaded_output, threaded_dir.path())
    );
}
