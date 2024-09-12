use chrono::NaiveDateTime;
use std::fs::File;

/// This function retrieves the date and time from the EXIF data of a file.
///
/// # Arguments
///
/// * `file` - A reference to a `File` object.
///
/// # Returns
///
/// * `Result<Option<NaiveDateTime>>` - A `Result` that, if `Ok`, contains an `Option` with the date and time from the EXIF data.
///   If the date and time could not be retrieved, the `Option` will be `None`.
///   If an error occurred during the process, the `Result` will be `Err`.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The file could not be read.
/// * The EXIF data could not be read from the file.
/// * The date and time could not be parsed from the EXIF data.
pub fn get_exif_time(file: &File) -> anyhow::Result<Option<NaiveDateTime>> {
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;
    let datetime = exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY);

    Ok(datetime
        .map(|field| {
            let datetime = field.display_value().to_string();
            NaiveDateTime::parse_from_str(&datetime, "%Y-%m-%d %H:%M:%S")
        })
        .transpose()?)
}
