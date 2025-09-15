use chrono::NaiveDateTime;
use std::io::{Read, Seek};
use std::str::FromStr;

/// The type of EXIF date to retrieve.
///
/// # Variants
///
/// * `Modify`: The modification date. When opening and saving a file, this is the date that is updated by the edit-software.
/// * `Creation`: The creation date. This is the date when the image was taken.
/// * `Digitized`: The digitized date. This is the date when the image was digitized. For example, when converting a film photo to a digital image. For digital cameras, this is usually the same as the creation date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExifDateType {
    /// The modification date. When opening and saving a file, this is the date that is updated by the edit-software.
    Modify,
    /// The creation date. This is the date when the image was taken.
    Creation,
    /// The digitized date. This is the date when the image was digitized. For example, when converting a film photo to a digital image. For digital cameras, this is usually the same as the creation date.
    Digitized,
}

impl FromStr for ExifDateType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "modify" | "m" | "modified" => Ok(ExifDateType::Modify),
            "creation" | "c" | "create" | "created" => Ok(ExifDateType::Creation),
            "digitized" | "digitize" | "d" | "digital" | "digitalize" => {
                Ok(ExifDateType::Digitized)
            }
            _ => Err(anyhow::anyhow!(
                "Invalid EXIF date type: {}. Possible values are modify/create/digitize",
                s
            )),
        }
    }
}

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
pub fn get_exif_time<R: Read + Seek>(
    file: R,
    date_type: ExifDateType,
) -> anyhow::Result<Option<NaiveDateTime>> {
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;
    let datetime = exif.get_field(
        match date_type {
            ExifDateType::Modify => exif::Tag::DateTime,
            ExifDateType::Creation => exif::Tag::DateTimeOriginal,
            ExifDateType::Digitized => exif::Tag::DateTimeDigitized,
        },
        exif::In::PRIMARY,
    );

    Ok(datetime
        .map(|field| {
            let datetime = field.display_value().to_string();
            NaiveDateTime::parse_from_str(&datetime, "%Y-%m-%d %H:%M:%S")
        })
        .transpose()?)
}
