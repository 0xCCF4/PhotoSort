//! Helpers for the unit and integration tests.
//!
//! This module is not part of the public API. It is compiled in every build so
//! that the binary tests and the integration tests in `tests/` can use it.

use crate::action::{ActionMode, ActualAction};
use crate::analysis::exif2date::ExifDateType;
use crate::analysis::name_formatters::BracketingFormattingPriority;
use crate::{build_file_patterns, AnalysisType, Analyzer, AnalyzerSettings};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counter that makes the names of the temporary directories unique.
static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Temporary directory that removes itself on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    /// Creates an empty temporary directory.
    ///
    /// `name` is only used to make the directory name readable.
    pub fn new(name: &str) -> Self {
        let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "photo_sort_test_{}_{}_{}",
            std::process::id(),
            name,
            counter
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temporary directory");
        TempDir(path)
    }

    /// Returns the path of the temporary directory.
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Creates an empty file and returns its path.
    pub fn create_file(&self, relative: &str) -> PathBuf {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        fs::write(&path, b"x").expect("Failed to create test file");
        path
    }

    /// Creates a symbolic link to a directory and returns the link path.
    pub fn symlink_dir(&self, target: &Path, link: &str) -> PathBuf {
        let link_path = self.0.join(link);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, &link_path).expect("Failed to create symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(target, &link_path).expect("Failed to create symlink");
        link_path
    }

    /// Creates a symbolic link to a file and returns the link path.
    pub fn symlink_file(&self, target: &Path, link: &str) -> PathBuf {
        let link_path = self.0.join(link);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, &link_path).expect("Failed to create symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(target, &link_path).expect("Failed to create symlink");
        link_path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Options for the [`analyzer_with`] helper.
#[derive(Debug, Default)]
pub struct AnalyzerOptions {
    /// The source directories. An empty list means that the target directory
    /// is the only source directory.
    pub source_dirs: Vec<PathBuf>,
    pub included_files: Vec<Regex>,
    pub excluded_files: Vec<Regex>,
    pub dont_follow_symlinked_dirs: bool,
    pub dont_follow_symlinked_files: bool,
}

/// Builds an `Analyzer` for the source directory `root`.
///
/// `root` is also used as the target directory.
pub fn analyzer(root: &Path, included_files: Vec<Regex>, excluded_files: Vec<Regex>) -> Analyzer {
    analyzer_with(
        root,
        AnalyzerOptions {
            included_files,
            excluded_files,
            ..Default::default()
        },
    )
}

/// Builds an `Analyzer` for `root` with the given options.
///
/// `root` is used as the target directory.
pub fn analyzer_with(root: &Path, options: AnalyzerOptions) -> Analyzer {
    let source_dirs = if options.source_dirs.is_empty() {
        vec![root.to_path_buf()]
    } else {
        options.source_dirs
    };
    Analyzer::new(AnalyzerSettings {
        analysis_type: AnalysisType::OnlyName,
        exif_date_type: ExifDateType::Creation,
        source_dirs,
        target_dir: root.to_path_buf(),
        recursive_source: true,
        dont_follow_symlinked_dirs: options.dont_follow_symlinked_dirs,
        dont_follow_symlinked_files: options.dont_follow_symlinked_files,
        file_format: String::new(),
        nodate_file_format: String::new(),
        unknown_file_format: None,
        bracketed_file_format: None,
        date_format: String::new(),
        bracketing_formatting: BracketingFormattingPriority::default(),
        extensions: Vec::new(),
        #[cfg(feature = "video")]
        video_extensions: Vec::new(),
        action_type: ActionMode::DryRun(ActualAction::Move),
        mkdir: false,
        excluded_files: options.excluded_files,
        included_files: options.included_files,
    })
    .expect("Failed to create analyzer")
}

/// Builds wildcard patterns without a regular expression part.
pub fn patterns(wildcards: &[&str]) -> Vec<Regex> {
    build_file_patterns(wildcards, [] as [&str; 0], false).expect("Invalid test pattern")
}

/// Scans the source directories of `analyzer` and returns the files sorted.
pub fn find(analyzer: &Analyzer, recursive: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut visited_paths = HashSet::new();
    for source in analyzer.settings.source_dirs.clone() {
        analyzer
            .find_files_in_source(source, recursive, &mut visited_paths, &mut files)
            .expect("Failed to scan source directory");
    }
    files.sort();
    files
}

/// Returns the file paths relative to `root`, sorted, with `/` as separator.
pub fn relative(files: &[PathBuf], root: &Path) -> Vec<String> {
    let root = root.canonicalize().expect("Failed to canonicalize root");
    let mut relative = files
        .iter()
        .map(|path| {
            path.strip_prefix(&root)
                .expect("File outside of source root")
                .display()
                .to_string()
                .replace(std::path::MAIN_SEPARATOR_STR, "/")
        })
        .collect::<Vec<String>>();
    relative.sort();
    relative
}

/// Builds a wildcard pattern from path parts, prefixed by the path separator.
pub fn path_pattern(parts: &[&str]) -> String {
    format!(
        "{}{}",
        std::path::MAIN_SEPARATOR_STR,
        parts.join(std::path::MAIN_SEPARATOR_STR)
    )
}

/// Path separator as it appears in a path pattern.
pub fn pattern_separator() -> &'static str {
    std::path::MAIN_SEPARATOR_STR
}

/// File name of the photo in the source root of the nested test tree.
pub const ROOT_PHOTO: &str = "a_20230102_101530.jpg";
/// File name of the photo in the `sub` directory of the nested test tree.
pub const SUB_PHOTO: &str = "b_20230102_101530.jpg";
/// File name of the photo in the `sub/nested` directory of the nested test tree.
pub const DEEP_PHOTO: &str = "c_20230102_101530.jpg";

/// Builds a source tree with one file in the root, one in `sub` and one in
/// `sub/nested`.
pub fn nested_tree(name: &str) -> TempDir {
    let dir = TempDir::new(name);
    dir.create_file(ROOT_PHOTO);
    dir.create_file(&format!("sub/{SUB_PHOTO}"));
    dir.create_file(&format!("sub/nested/{DEEP_PHOTO}"));
    dir
}
