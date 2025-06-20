use anyhow::anyhow;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use regex::{Captures, Regex};
use std::sync::LazyLock;

static RE_NAIVE_FILENAME: LazyLock<Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(\d{4})[-_]?(\d{2})[-_]?(\d{2})(\D+(\d{2})[-_:]?(\d{2})[-_:]?(\d{2}))?[-_]?",
    )
    .expect("Failed to compile regex")
});

#[derive(Debug, Default)]
/// A `FileNameToDateTransformer` implementation that extracts the date from a file name in the format
/// `YYYY[-_]?MM\[-_]?DD\[-_]?(HH[-_:]?MM[-_:]?SS)?`.
pub struct NaiveFileNameParser {}

impl FileNameToDateTransformer for NaiveFileNameParser {
    fn get_regex(&self) -> &Regex {
        &RE_NAIVE_FILENAME
    }

    fn transform(&self, capture: &Captures) -> anyhow::Result<Option<NaiveDateTime>> {
        let year = capture
            .get(1)
            .ok_or_else(|| anyhow!("Regex did not find year group"))?
            .as_str()
            .parse::<i32>()?;
        let month = capture
            .get(2)
            .ok_or_else(|| anyhow!("Regex did not find month group"))?
            .as_str()
            .parse::<u32>()?;
        let day = capture
            .get(3)
            .ok_or_else(|| anyhow!("Regex did not find day group"))?
            .as_str()
            .parse::<u32>()?;

        let time = if let (Some(hour), Some(minute), Some(second)) =
            (capture.get(5), capture.get(6), capture.get(7))
        {
            NaiveTime::from_hms_opt(
                hour.as_str().parse::<u32>()?,
                minute.as_str().parse::<u32>()?,
                second.as_str().parse::<u32>()?,
            )
            .ok_or_else(|| anyhow!("Invalid time"))?
        } else {
            // no time given, assume 00:00
            NaiveTime::MIN
        };

        let date =
            NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| anyhow!("Invalid date"))?;
        Ok(Some(NaiveDateTime::new(date, time)))
    }
}

/// `NameTransformer` is a struct that represents a transformer to convert a file name into a `NaiveDateTime`.
///
/// This is done in two steps:
/// 1. A regular expression is matched against the file name.
/// 2. If the regular expression matches: Call the transformer function
pub trait FileNameToDateTransformer {
    /// Gets the regular expression to match against the file name.
    ///
    /// # Returns
    /// A reference to the regular expression.
    fn get_regex(&self) -> &regex::Regex;

    /// This function is called if the regular expression matches the file name.
    ///
    /// # Arguments
    /// * `capture` - A reference to a `regex::Captures` object that contains the matched groups.
    ///
    /// # Returns
    /// * `NaiveDateTime` - If the date and time could be extracted from the matched groups.
    /// * `None` - If the transformer decides that it cant extract a date but no error occurred. The next transformer will be called.
    ///
    /// # Errors
    /// If transformation fails, an error is returned. The program will log the error and try the next transformer.
    fn transform(&self, capture: &regex::Captures) -> anyhow::Result<Option<NaiveDateTime>>;

    /// Tries to transform a file name into a date and time.
    ///
    /// # Errors
    /// If transformation fails, an error is returned. The program will log the error and try the next transformer.
    fn try_transform_name(&self, name: &str) -> anyhow::Result<Option<(NaiveDateTime, String)>> {
        let all_matches = self.get_regex().captures_iter(name);
        for current_match in all_matches {
            let matched = current_match.get(0).map_or("", |m| m.as_str());
            match self.transform(&current_match) {
                Ok(Some(dt)) => return Ok(Some((dt, name.replace(matched, "")))),
                Ok(None) => {}
                Err(e) => {
                    log::error!("Error: {e:?}");
                }
            }
        }
        Ok(None) // No match found
    }
}
