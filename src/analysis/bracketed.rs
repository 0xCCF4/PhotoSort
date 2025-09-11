use crate::BracketEXIFInformation;
use anyhow::anyhow;
use std::path::Path;

mod sony;

/// Analyzes the EXIF data of the specified file on its "bracketing" state
/// Multiple photos may belong to the same "group" called bracketed. This info
/// is stored in manufacturer specific EXIF information
///
/// # Arguments
///
/// * `photo_path` - Path to the photo to analyze
///
/// # Returns
///
/// * `Result<Option<BracketEXIFInformation>>` - Returns the bracketing sequence number of None
///
/// # Errors
///
/// * On File IO Error
pub fn get_bracketing_info<P: AsRef<Path>>(
    photo_path: P,
) -> anyhow::Result<Option<BracketEXIFInformation>> {
    let file =
        std::fs::File::open(photo_path).map_err(|e| anyhow!("Error while opening file: {e}"))?;
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader
        .read_from_container(&mut bufreader)
        .map_err(|e| anyhow!("Error while reading EXIF {e}"))?;

    let Some(x) = exif.get_field(exif::Tag::MakerNote, exif::In::PRIMARY) else {
        return Ok(None);
    };
    let exif::Value::Undefined(value, _) = &x.value else {
        return Ok(None);
    };

    sony::get_bracketing_info(value)
}
