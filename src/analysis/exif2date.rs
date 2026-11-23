use crate::analysis::time_zone::{parse_offset, read_in_offset, TimeZone, ZonedDateTime};
use chrono::{FixedOffset, NaiveDateTime};
use log::trace;
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

impl ExifDateType {
    /// Returns the tag of the date field and the tag of its offset field.
    fn tags(self) -> (exif::Tag, exif::Tag) {
        match self {
            ExifDateType::Modify => (exif::Tag::DateTime, exif::Tag::OffsetTime),
            ExifDateType::Creation => (exif::Tag::DateTimeOriginal, exif::Tag::OffsetTimeOriginal),
            ExifDateType::Digitized => {
                (exif::Tag::DateTimeDigitized, exif::Tag::OffsetTimeDigitized)
            }
        }
    }
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
                "Invalid EXIF date type: {s}. Possible values are modify/create/digitize"
            )),
        }
    }
}

/// Returns the date and time of a photo from its EXIF data. `file` holds the photo and `date_type`
/// selects which EXIF date field to read. A value with an EXIF offset uses that offset. A value
/// without one is read in `zone`. When the EXIF data holds a date of the requested kind, the result
/// is `Some`, and `None` otherwise. When the file or its EXIF data cannot be read, or when the date
/// cannot be parsed, the function returns an error.
pub fn get_exif_time<R: Read + Seek>(
    file: R,
    date_type: ExifDateType,
    zone: TimeZone,
) -> anyhow::Result<Option<ZonedDateTime>> {
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;
    let (date_tag, offset_tag) = date_type.tags();
    let Some(field) = exif.get_field(date_tag, exif::In::PRIMARY) else {
        return Ok(None);
    };

    let datetime = field.display_value().to_string();
    let time = NaiveDateTime::parse_from_str(&datetime, "%Y-%m-%d %H:%M:%S")?;

    let offset = exif
        .get_field(offset_tag, exif::In::PRIMARY)
        .and_then(offset_value);

    Ok(Some(match offset {
        Some(offset) => {
            trace!("Using EXIF offset {offset} of {date_tag} for {datetime}");
            read_in_offset(time, offset)
        }
        None => zone.read(time),
    }))
}

/// Returns the offset that an EXIF offset field states, for example `+01:00`.
fn offset_value(field: &exif::Field) -> Option<FixedOffset> {
    let exif::Value::Ascii(values) = &field.value else {
        return None;
    };
    let text = std::str::from_utf8(values.first()?).ok()?;
    parse_offset(text.trim_end_matches('\0'))
}
