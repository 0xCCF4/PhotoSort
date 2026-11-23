use anyhow::{anyhow, Result};
use filetime::FileTime;
use log::{debug, error, info, trace};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// `ActualAction` is an enumeration that defines the different types of actions that can be performed on a file.
///
/// # Variants
///
/// * `Move` - Represents the action of moving a file.
/// * `Copy` - Represents the action of copying a file.
/// * `Hardlink` - Represents the action of creating a hard link to a file.
/// * `RelativeSymlink` - Represents the action of creating a relative symbolic link to a file.
/// * `AbsoluteSymlink` - Represents the action of creating an absolute symbolic link to a file.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ActualAction {
    Move,
    Copy,
    Hardlink,
    RelativeSymlink,
    AbsoluteSymlink,
}

impl Display for ActualAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActualAction::Move => write!(f, "Move"),
            ActualAction::Copy => write!(f, "Copy"),
            ActualAction::Hardlink => write!(f, "Hardlink"),
            ActualAction::RelativeSymlink => write!(f, "RelSymlink"),
            ActualAction::AbsoluteSymlink => write!(f, "AbsSymlink"),
        }
    }
}

/// Log target of the dry-run plan lines.
pub const PLAN_TARGET: &str = "photo_sort::plan";

/// `ActionMode` defines the mode of operation of the tool
///
/// # Variants
/// * `Execute` - The provided action will be executed
/// * `DryRun` - The provided action will be printed but not executed
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ActionMode {
    Execute(ActualAction),
    DryRun(ActualAction),
}

/// `FromStr` trait implementation for `ActualAction`.
///
/// This allows a string to be parsed into the `ActualAction` enum.
///
/// # Arguments
///
/// * `s` - A string slice that should be parsed into an `ActualAction`.
///
/// # Returns
///
/// * `Result<Self, Self::Err>` - Returns `Ok(ActualAction)` if the string could be parsed into an `ActionMode`, `Err(anyhow::Error)` otherwise.
impl FromStr for ActualAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "move" => Ok(ActualAction::Move),
            "copy" => Ok(ActualAction::Copy),
            "hardlink" | "hard" => Ok(ActualAction::Hardlink),
            "relative_symlink" | "relsym" => Ok(ActualAction::RelativeSymlink),
            "absolute_symlink" | "abssym" => Ok(ActualAction::AbsoluteSymlink),
            _ => Err(anyhow::anyhow!("Invalid action mode")),
        }
    }
}

/// Performs the specified action on the source file and target file.
///
/// # Arguments
///
/// * `source` - A `PathBuf` reference to the source file.
/// * `target` - A `PathBuf` reference to the target file.
/// * `action` - An `ActionMode` reference specifying the action to be performed.
/// * `mkdir` - Mkdir subfolders on the way, in dry-run mode no subfolders are created.
///
/// # Returns
///
/// * `std::io::Result<()>` - An IO Result indicating the success or failure of the operation.
///
/// # Actions
///
/// * `ActionMode::DryRun` - Prints the operation that would be performed without actually performing it.
/// * `ActionMode::Execute` - Performs the specified action on the source file and target file.
///    * `ActualAction::Move` - Moves the source file to the target location.
///    * `ActualAction::Copy` - Copies the source file to the target location.
///    * `ActualAction::Hardlink` - Creates a hard link at the target location pointing to the source file.
///    * `ActualAction::RelativeSymlink` - Creates a relative symbolic link at the target location pointing to the source file.
///    * `ActualAction::AbsoluteSymlink` - Creates an absolute symbolic link at the target location pointing to the source file.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The target file already exists.
/// * An error occurred during the file operation.
pub fn file_action<P: AsRef<Path>, Q: AsRef<Path>>(
    source: P,
    target: Q,
    action: &ActionMode,
    mkdir: bool,
) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    if matches!(action, ActionMode::DryRun(_)) {
        error_file_exists(target)
            .map_err(|e| anyhow!("Target file already exists: {target:?} - {e:?}"))?;
    }

    // check if parent folder exists
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            if !mkdir {
                return Err(anyhow!(
                    "Target subfolder does not exist. Use --mkdir to create it: {parent:?}"
                ));
            }

            if matches!(action, ActionMode::DryRun(_)) {
                info!(target: PLAN_TARGET, "[Mkdir] {}", parent.display());
            } else {
                fs::create_dir_all(parent).map_err(|e| {
                    anyhow!("Failed to create target subfolder: {parent:?} - {e:?}")
                })?;
            }
        }
    }

    match action {
        ActionMode::Execute(ActualAction::Move) => move_file(source, target),
        ActionMode::Execute(ActualAction::Copy) => copy_file(source, target),
        ActionMode::Execute(ActualAction::Hardlink) => hardlink_file(source, target),
        ActionMode::Execute(ActualAction::RelativeSymlink) => relative_symlink_file(source, target),
        ActionMode::Execute(ActualAction::AbsoluteSymlink) => absolute_symlink_file(source, target),
        ActionMode::DryRun(action) => {
            dry_run(source, target, *action);
            Ok(())
        }
    }
}

fn dry_run<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B, action: ActualAction) {
    info!(
        target: PLAN_TARGET,
        "[{}] {} -> {}",
        action,
        source.as_ref().display(),
        target.as_ref().display()
    );
}

fn error_file_exists(target: &Path) -> std::io::Result<()> {
    if target.exists() {
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Target file already exists",
        ))
    } else {
        Ok(())
    }
}

/// Claims a target path for this process.
///
/// # Errors
///
/// Returns an error if the path exists. The plan computed this name for the
/// source, so a file that is already there was created after the plan was built,
/// or the file system does not separate the two names. Nothing is overwritten in
/// either case.
fn reserve_target(target: &Path) -> Result<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
    {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(anyhow!(
            "Target file already exists: {}. Nothing was overwritten. Another process created it after the plan was built, or the file system does not separate the two names.",
            target.display()
        )),
        Err(err) => Err(anyhow!(
            "Failed to create the target file {}: {err}",
            target.display()
        )),
    }
}

fn copy_file<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    debug!("Copying {} -> {}", source.display(), target.display());

    reserve_target(target)?;

    if let Err(err) = copy_into_reserved(source, target) {
        let _ = fs::remove_file(target);
        return Err(anyhow!(
            "Failed to copy {} to {}: {err}",
            source.display(),
            target.display()
        ));
    }

    Ok(())
}

/// Copies `source` over the reserved (and still empty) file `target` and applies
/// the timestamps of the source file.
fn copy_into_reserved<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B) -> std::io::Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    let metadata = fs::metadata(source)?;
    let result = fs::copy(source, target)?;

    if metadata.len() != result {
        return Err(std::io::Error::other("File copy failed"));
    }

    let mtime = FileTime::from_last_modification_time(&metadata);
    let atime = FileTime::from_last_access_time(&metadata);

    filetime::set_file_times(target, atime, mtime)?;

    Ok(())
}

fn move_file<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    debug!("Moving {} -> {}", source.display(), target.display());

    reserve_target(target)?;

    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(err) => {
            trace!(
                "Renaming file failed, falling back to cut/paste (maybe cross-fs rename operation?): {:?} for file {} -> {}",
                err,
                source.display(),
                target.display()
            );

            if let Err(err) = copy_into_reserved(source, target) {
                let _ = fs::remove_file(target);
                return Err(anyhow!(
                    "Failed to copy {} to {}: {err}",
                    source.display(),
                    target.display()
                ));
            }

            fs::remove_file(source).map_err(|err| {
                anyhow!(
                    "The file was copied to {} but the source {} could not be removed: {err}",
                    target.display(),
                    source.display()
                )
            })
        }
    }
}

fn hardlink_file<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    debug!(
        "Creating hardlink {} -> {}",
        source.display(),
        target.display()
    );

    let result = fs::hard_link(source, target);
    if let Err(err) = result {
        error!(
            "Creating hardlink failed, falling back to copy: {:?} for file {} -> {}",
            err,
            source.display(),
            target.display()
        );
        copy_file(source, target)
    } else {
        Ok(())
    }
}

fn relative_symlink_file<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    debug!(
        "Creating symlink {} -> {}",
        source.display(),
        target.display()
    );

    symlink::symlink_file(source, target).map_err(|err| {
        anyhow!(
            "Failed to create the symbolic link {} -> {}: {err}",
            target.display(),
            source.display()
        )
    })
}

fn absolute_symlink_file<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    debug!(
        "Creating symlink {} -> {}",
        source.display(),
        target.display()
    );
    let source = fs::canonicalize(source)
        .map_err(|err| anyhow!("Failed to resolve the path {}: {err}", source.display()))?;

    relative_symlink_file(&source, target)
}
