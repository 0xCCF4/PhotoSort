//! Computation of all target paths before the first file is touched.
//!
//! The target name of a file depends on its analysis, so the analysis runs
//! first, and in multi-threaded mode it runs in parallel. The plan then resolves
//! the name collisions in a single thread. This has two effects:
//!
//! * The de-duplication decision never sees a target file that is still being
//!   written, because no file is written while the plan is built. Two source
//!   files are compared against each other while both are complete.
//! * The result does not depend on the order in which the analyses finished.
//!   The plan is built in the sorted order of the source paths, so the
//!   `{dup}` numbers are stable between runs.
//!
//! Only after the plan exists are the file actions executed, in parallel.

use crate::action::{ActionMode, ActualAction};
use crate::analysis::name_formatters::{BracketInfo, NameFormatterInvocationInfo};
use crate::{streams_equal, Analyzer};
use anyhow::{anyhow, Result};
use log::{debug, info, trace};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// One analysed file, ready to be planned.
pub struct AnalyzedFile {
    /// The source file.
    pub source: PathBuf,
    /// The result of the analysis of the source file.
    pub info: NameFormatterInvocationInfo<'static>,
    /// The bracketing information, if the file belongs to a bracketed photo set.
    pub bracket: Option<BracketInfo<'static>>,
}

/// The action that the plan assigns to a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanKind {
    /// Perform the configured action from the source to the target.
    Action,
    /// The source has the same content as the target. Nothing is written. In
    /// move mode the source file is removed, in all other modes it is left
    /// alone.
    DropDuplicate,
}

/// One line of the plan.
#[derive(Debug, Clone)]
pub struct PlanEntry {
    /// The source file.
    pub source: PathBuf,
    /// The target file.
    pub target: PathBuf,
    /// What to do with the source file.
    pub kind: PlanKind,
}

/// The plan for one run.
#[derive(Debug, Default)]
pub struct Plan {
    /// The actions to execute.
    pub entries: Vec<PlanEntry>,
    /// Files that could not be planned, with the reason.
    pub errors: Vec<(PathBuf, String)>,
}

/// One group of source files that have the same content.
struct ContentClass {
    /// A source file that represents the content of this class.
    representative: PathBuf,
}

/// All content classes that claim one target path.
#[derive(Default)]
struct TargetSlot {
    classes: Vec<ContentClass>,
}

/// Where the planner looks up whether a target path is taken.
enum Existing {
    /// Batch mode: one `read_dir` snapshot per target directory, taken on
    /// demand. Only another process can change the directory after that.
    Snapshot(HashMap<PathBuf, HashSet<OsString>>),
    /// Streaming mode: ask the file system for every candidate path.
    Directory,
}

/// Computes target paths for a run and remembers which paths are already
/// claimed.
///
/// In batch mode the caller analyses all files first, then calls
/// [`Planner::plan`]. In streaming mode the caller analyses one file at a time
/// and calls [`Planner::resolve_one`] for each of them.
pub struct Planner<'a> {
    analyzer: &'a Analyzer,
    existing: Existing,
    slots: HashMap<PathBuf, TargetSlot>,
}

impl<'a> Planner<'a> {
    /// Creates a planner for a batch of files. The directories of the target
    /// tree are read once per directory, when the first file needs them.
    pub fn batch(analyzer: &'a Analyzer) -> Planner<'a> {
        Planner {
            analyzer,
            existing: Existing::Snapshot(HashMap::new()),
            slots: HashMap::new(),
        }
    }

    /// Creates a planner for a single file that is analysed and executed right
    /// away. The file system is asked for every candidate path.
    pub fn streaming(analyzer: &'a Analyzer) -> Planner<'a> {
        Planner {
            analyzer,
            existing: Existing::Directory,
            slots: HashMap::new(),
        }
    }

    /// Computes the plan for a batch of files. The order of `files` decides the
    /// `{dup}` numbers, so sort the files by source path before calling this if
    /// the result must be reproducible.
    pub fn plan(&mut self, files: Vec<AnalyzedFile>) -> Plan {
        let mut plan = Plan::default();

        for file in files {
            let AnalyzedFile {
                source,
                info,
                bracket,
            } = file;
            // The analysis result is owned, the bracketing information is not.
            // Shortening the lifetime of the analysis result here is enough to
            // attach the bracketing information.
            let mut info: NameFormatterInvocationInfo<'_> = info;
            info.bracket_info = bracket.as_ref();

            match self.resolve_one(&source, &mut info) {
                Ok(entry) => plan.entries.push(entry),
                Err(err) => plan.errors.push((source, err.to_string())),
            }
        }

        plan
    }

    /// Computes the target path for one file and claims it.
    ///
    /// A candidate path is tried in ascending `{dup}` order. If the candidate is
    /// claimed by the plan, the source is compared with the source that claimed
    /// it. If the candidate exists on disk, the source is compared with the file
    /// on disk. In both cases identical content means the source is a duplicate
    /// and nothing is written for it.
    ///
    /// # Errors
    ///
    /// Returns an error if the format string ignores `{dup}` and the first
    /// candidate path is taken by content that differs.
    pub fn resolve_one(
        &mut self,
        source: &Path,
        info: &mut NameFormatterInvocationInfo,
    ) -> Result<PlanEntry> {
        let mut counter = 0u32;
        let mut previous: Option<PathBuf> = None;

        loop {
            info.duplicate_counter = (counter > 0).then_some(counter);
            let candidate = self.analyzer.target_path(info)?;

            if previous.as_ref() == Some(&candidate) {
                return Err(anyhow!(
                    "Target file {} already exists; content differs. Consider providing the {{:dup}} format specifier to the format string to resolve duplicate naming.",
                    candidate.display()
                ));
            }

            if self.slots.contains_key(&candidate) {
                if self.compare_with_slot(source, &candidate)? {
                    debug!(
                        "De-duplicating file {} because it is identical to the planned target file {}.",
                        source.display(),
                        candidate.display()
                    );
                    return Ok(PlanEntry {
                        source: source.to_path_buf(),
                        target: candidate,
                        kind: PlanKind::DropDuplicate,
                    });
                }

                trace!(
                    "Target file {} is claimed by a file with different content, trying the next name",
                    candidate.display()
                );
                previous = Some(candidate);
                counter += 1;
                continue;
            }

            if self.exists(&candidate) {
                if same_content(&candidate, source)? {
                    debug!(
                        "De-duplicating file {} because it is identical to the target file {}.",
                        source.display(),
                        candidate.display()
                    );
                    return Ok(PlanEntry {
                        source: source.to_path_buf(),
                        target: candidate,
                        kind: PlanKind::DropDuplicate,
                    });
                }

                trace!(
                    "Target file {} exists with different content, trying the next name",
                    candidate.display()
                );
                previous = Some(candidate);
                counter += 1;
                continue;
            }

            self.slots.insert(
                candidate.clone(),
                TargetSlot {
                    classes: vec![ContentClass {
                        representative: source.to_path_buf(),
                    }],
                },
            );

            if counter > 0 {
                info!("De-duplicated target file: {}", candidate.display());
            }

            return Ok(PlanEntry {
                source: source.to_path_buf(),
                target: candidate,
                kind: PlanKind::Action,
            });
        }
    }

    /// Compares `source` with every content class that claims `target`. Returns
    /// `true` and records the source if the content of one class matches.
    fn compare_with_slot(&mut self, source: &Path, target: &Path) -> Result<bool> {
        let Some(slot) = self.slots.get_mut(target) else {
            return Ok(false);
        };

        for class in &mut slot.classes {
            if same_content(&class.representative, source)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Returns whether `target` exists. In batch mode the answer comes from a
    /// snapshot of the directory, which is read once.
    fn exists(&mut self, target: &Path) -> bool {
        let (Some(parent), Some(name)) = (target.parent(), target.file_name()) else {
            return target.exists();
        };

        match &mut self.existing {
            Existing::Directory => target.exists(),
            Existing::Snapshot(cache) => {
                let names = cache.entry(parent.to_path_buf()).or_insert_with(|| {
                    match fs::read_dir(parent) {
                        Ok(entries) => entries
                            .filter_map(std::result::Result::ok)
                            .map(|entry| entry.file_name())
                            .collect(),
                        // The directory does not exist yet. It is created during
                        // execution if --mkdir is set.
                        Err(_) => HashSet::new(),
                    }
                });

                names.contains(name)
            }
        }
    }
}

/// Compares the content of two complete files.
///
/// The size is compared first, and a hardlink is recognised by its device and
/// inode number, so the content is only read when both say nothing.
fn same_content(a: &Path, b: &Path) -> io::Result<bool> {
    let meta_a = fs::metadata(a)?;
    let meta_b = fs::metadata(b)?;

    if meta_a.len() != meta_b.len() {
        return Ok(false);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta_a.dev() == meta_b.dev() && meta_a.ino() == meta_b.ino() {
            return Ok(true);
        }
    }

    streams_equal(File::open(a)?, File::open(b)?)
}

impl Analyzer {
    /// Executes one line of the plan.
    ///
    /// # Errors
    ///
    /// Returns an error if the action fails or if the source of a duplicate
    /// cannot be removed in move mode.
    pub fn execute_entry(&self, entry: &PlanEntry) -> Result<()> {
        match entry.kind {
            PlanKind::Action => crate::action::file_action(
                &entry.source,
                &entry.target,
                &self.settings.action_type,
                self.settings.mkdir,
            ),
            PlanKind::DropDuplicate => match self.settings.action_type {
                ActionMode::Execute(ActualAction::Move) => {
                    fs::remove_file(&entry.source).map_err(|err| {
                        anyhow!(
                            "Failed to remove source file during de-duplication: {}: {err}",
                            entry.source.display()
                        )
                    })
                }
                ActionMode::Execute(
                    ActualAction::Copy
                    | ActualAction::Hardlink
                    | ActualAction::RelativeSymlink
                    | ActualAction::AbsoluteSymlink,
                )
                | ActionMode::DryRun(_) => Ok(()),
            },
        }
    }
}
