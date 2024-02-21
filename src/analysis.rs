use std::fs::File;
use exif;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use regex;
use anyhow::Result;
use anyhow::anyhow;

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
pub fn get_exif_time(file: &File) -> Result<Option<NaiveDateTime>> {
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;
    let datetime = exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY);

    Ok(datetime.map(|field| {
        let datetime = field.display_value().to_string();
        NaiveDateTime::parse_from_str(&datetime, "%Y-%m-%d %H:%M:%S")
    }).transpose()?)
}

/// `NameTransformer` is a struct that represents a transformer to convert a file name into a `NaiveDateTime`.
///
/// It contains a regular expression to match the name and a transformation function to convert the matched part into a `NaiveDateTime`.
///
/// # Fields
///
/// * `regex` - A `regex::Regex` that represents the regular expression to match the name.
/// * `transform` - A function that takes a `regex::Captures` and returns a `Result<NaiveDateTime>`. This function is used to convert the matched phrase into a `NaiveDateTime`.
pub struct NameTransformer {
    regex: regex::Regex,
    transform: fn(regex::Captures) -> Result<NaiveDateTime>,
}

impl NameTransformer {
    /// Constructs a new `NameTransformer` instance.
    ///
    /// # Arguments
    ///
    /// * `regex` - A `regex::Regex` that represents the regular expression to match the name.
    /// * `transform` - A function that takes a `regex::Captures` and returns a `Result<NaiveDateTime>`. This function is used to convert the matched phrase into a `NaiveDateTime`.
    ///
    /// # Returns
    ///
    /// * `NameTransformer` - A new `NameTransformer` instance.
    pub fn new(regex: regex::Regex, transform: fn(regex::Captures) -> Result<NaiveDateTime>) -> NameTransformer {
        NameTransformer {
            regex,
            transform,
        }
    }

    /// This function generates a vector of `NameTransformer` instances.
    ///
    /// Each `NameTransformer` instance contains a regular expression and a transformation function.
    /// The regular expression is used to match a specific pattern in a file name.
    /// The transformation function is used to convert the matched string into a `NaiveDateTime`.
    /// The result will contain a standard list of `NameTransformer` instances that can be used to parse file names.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<NameTransformer>>` - A `Result` that, if `Ok`, contains a vector of `NameTransformer` instances.
    ///   If an error occurred during the process, the `Result` will be `Err`.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// * The regular expression could not be compiled.
    pub fn get_standard_name_parsers() -> Result<Vec<NameTransformer>> {
        let p = NameTransformer {
            regex: regex::Regex::new(r"(\d{4})[-_]?(\d{2})[-_]?(\d{2})(\D+(\d{2})[-_:]?(\d{2})[-_:]?(\d{2}))?[-_]?")?,

            transform: |m| {
                let year = m.get(1).ok_or_else(|| anyhow!("Regex did not find year group"))?.as_str().parse::<i32>()?;
                let month = m.get(2).ok_or_else(|| anyhow!("Regex did not find month group"))?.as_str().parse::<u32>()?;
                let day = m.get(3).ok_or_else(|| anyhow!("Regex did not find day group"))?.as_str().parse::<u32>()?;

                let time = if let (Some(hour), Some(minute), Some(second)) = (m.get(5), m.get(6), m.get(7)) {
                    NaiveTime::from_hms_opt(
                        hour.as_str().parse::<u32>()?,
                        minute.as_str().parse::<u32>()?,
                        second.as_str().parse::<u32>()?,
                    ).ok_or_else(|| anyhow!("Invalid time"))?
                } else {
                    NaiveTime::MIN
                };

                let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| anyhow!("Invalid date"))?;
                Ok(NaiveDateTime::new(date, time))
            },
        };

        Ok(vec![p])
    }
}

/// This function tries to retrieve a photo creation date and time from a file name.
///
/// The function accepts a list of `NameTransformer` instances that are used to match and transform the file name into a datetime.
/// Each `NameTransformer` instance contains a regular expression and a transformation function.
/// A list of standard `NameTransformer` instances can be generated using the `NameTransformer::get_standard_name_parsers` function.
///
/// # Arguments
///
/// * `name` - A reference to a string that represents the file name.
/// * `parsers` - A reference to a vector of `NameTransformer` instances.
///
/// # Returns
///
/// * `Result<Option<(NaiveDateTime, String)>>` - A `Result` that, if `Ok`, contains an `Option` with a tuple.
///   The first element of the tuple is the date and time from the file name.
///   The second element of the tuple is the file name with the matched part removed.
///   If the date and time could not be retrieved, the `Option` will be `None`.
///   If an error occurred during the process, the `Result` will be `Err`.
///
/// # Errors
///
/// This function will return an error if:
///
/// * A transformation function failed and errors
pub fn get_name_time(name: &str, parsers: &Vec<NameTransformer>) -> Result<Option<(NaiveDateTime, String)>> {
    for phrase in parsers {
        let iter = phrase.regex.captures_iter(name);
        for cap in iter {
            let matched = cap.get(0).map_or("", |m| m.as_str());
            match (phrase.transform)(cap) {
                Ok(dt) => return Ok(Some(
                    (dt, name.replace(matched, "")))),
                Err(e) => {
                    log::error!("Error: {:?}", e);
                },
            }
        }
    }

    Ok(None)
}
