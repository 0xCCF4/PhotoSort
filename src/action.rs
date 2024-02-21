use std::{fs};
use std::path::PathBuf;
use std::str::FromStr;
use filetime::FileTime;
use log::{debug, error, warn};

/// `ActionMode` is an enumeration that defines the different types of actions that can be performed on a file.
///
/// # Variants
///
/// * `Move` - Represents the action of moving a file.
/// * `Copy` - Represents the action of copying a file.
/// * `Hardlink` - Represents the action of creating a hard link to a file.
/// * `RelativeSymlink` - Represents the action of creating a relative symbolic link to a file.
/// * `AbsoluteSymlink` - Represents the action of creating an absolute symbolic link to a file.
/// * `DryRun` - Represents a dry run action, which prints the operation that would be performed without actually performing it.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ActionMode {
    Move,
    Copy,
    Hardlink,
    RelativeSymlink,
    AbsoluteSymlink,
    DryRun,
}

/// `FromStr` trait implementation for `ActionMode`.
///
/// This allows a string to be parsed into the `ActionMode` enum.
///
/// # Arguments
///
/// * `s` - A string slice that should be parsed into an `ActionMode`.
///
/// # Returns
///
/// * `Result<Self, Self::Err>` - Returns `Ok(ActionMode)` if the string could be parsed into an `ActionMode`, `Err(anyhow::Error)` otherwise.
impl FromStr for ActionMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "move" => Ok(ActionMode::Move),
            "copy" => Ok(ActionMode::Copy),
            "hardlink" => Ok(ActionMode::Hardlink),
            "hard" => Ok(ActionMode::Hardlink), // Alias for "Hardlink"
            "relative_symlink" => Ok(ActionMode::RelativeSymlink),
            "relsym" => Ok(ActionMode::RelativeSymlink), // Alias for "RelativeSymlink"
            "absolute_symlink" => Ok(ActionMode::AbsoluteSymlink),
            "abssym" => Ok(ActionMode::AbsoluteSymlink), // Alias for "AbsoluteSymlink"
            _ => Err(anyhow::anyhow!("Invalid action mode"))
        }
    }
}

/// Performs the specified action on the source file and target file.
///
/// # Arguments
///
/// * `source` - A PathBuf reference to the source file.
/// * `target` - A PathBuf reference to the target file.
/// * `action` - An ActionMode reference specifying the action to be performed.
///
/// # Returns
///
/// * `std::io::Result<()>` - An IO Result indicating the success or failure of the operation.
///
/// # Actions
///
/// * `ActionMode::Move` - Moves the source file to the target location.
/// * `ActionMode::Copy` - Copies the source file to the target location.
/// * `ActionMode::Hardlink` - Creates a hard link at the target location pointing to the source file.
/// * `ActionMode::RelativeSymlink` - Creates a relative symbolic link at the target location pointing to the source file.
/// * `ActionMode::AbsoluteSymlink` - Creates an absolute symbolic link at the target location pointing to the source file.
/// * `ActionMode::DryRun` - Prints the operation that would be performed without actually performing it.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The target file already exists.
/// * An error occurred during the file operation.
pub fn file_action(source: &PathBuf, target: &PathBuf, action: &ActionMode) -> std::io::Result<()> {
    error_file_exists(target)?;
    match action {
        ActionMode::Move => move_file(source, target),
        ActionMode::Copy => copy_file(source, target),
        ActionMode::Hardlink => hardlink_file(source, target),
        ActionMode::RelativeSymlink => relative_symlink_file(source, target),
        ActionMode::AbsoluteSymlink => absolute_symlink_file(source, target),
        ActionMode::DryRun => dry_run(source, target),
    }
}

fn dry_run(source: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    println!("Copying {:?} -> {:?}", source, target);
    Ok(())
}

fn error_file_exists(target: &PathBuf) -> std::io::Result<()> {
    if target.exists() {
        Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Target file already exists"))
    } else {
        Ok(())
    }
}

fn copy_file(source: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    debug!("Copying {:?} -> {:?}", source, target);

    let metadata = fs::metadata(source)?;
    let result = fs::copy(source, target)?;

    if metadata.len() != result {
        let _ = fs::remove_file(target);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "File copy failed"));
    }

    let mtime = FileTime::from_last_modification_time(&metadata);
    let atime = FileTime::from_last_access_time(&metadata);

    filetime::set_file_times(target, atime, mtime)?;

    Ok(())
}

fn move_file(source: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    debug!("Moving {:?} -> {:?}", source, target);

    let result = fs::rename(source, target);
    if let Err(err) = result {
        warn!("Renaming file failed, falling back to cut/paste: {:?} for file {:?} -> {:?}", err, source, target);
        copy_file(source, target)?;
        fs::remove_file(source)
    } else {
        Ok(())
    }
}

fn hardlink_file(source: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    debug!("Creating hardlink {:?} -> {:?}", source, target);

    let result = fs::hard_link(source, target);
    if let Err(_err) = result {
        error!("Creating hardlink failed, falling back to copy: {:?} for file {:?} -> {:?}", _err, source, target);
        copy_file(source, target)
    } else {
        Ok(())
    }
}

fn relative_symlink_file(source: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    debug!("Creating symlink {:?} -> {:?}", source, target);

    symlink::symlink_file(source, target)?;

    Ok(())
}

fn absolute_symlink_file(source: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    debug!("Creating symlink {:?} -> {:?}", source, target);
    let source = fs::canonicalize(source)?;

    relative_symlink_file(&source, &target)
}