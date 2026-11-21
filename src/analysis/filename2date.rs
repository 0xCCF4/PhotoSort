use anyhow::anyhow;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use regex::{Captures, Regex};
use std::ops::RangeInclusive;
use std::sync::LazyLock;

static RE_NAIVE_FILENAME: LazyLock<Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?P<date>(?P<year>\d{4})[-_]?(?P<month>\d{2})[-_]?(?P<day>\d{2}))(\D+(?P<hour>\d{2})[-_:]?(?P<minute>\d{2})[-_:]?(?P<second>\d{2}))?[-_]?",
    )
    .expect("Failed to compile regex")
});

/// Years that are accepted as part of a date found in a file name.
///
/// Any sequence of eight digits looks like a date to the regular expression, for example the
/// first digits of a Unix timestamp (`FB_IMG_1612101530123.jpg`) or of a camera counter
/// (`DSC10201231.jpg`). Digits that would result in a year outside of this range are therefore
/// not interpreted as a date.
const PLAUSIBLE_YEARS: RangeInclusive<i32> = 1900..=2100;

#[derive(Debug, Default)]
/// A `FileNameToDateTransformer` implementation that extracts the date from a file name in the format
/// `YYYY[-_]?MM\[-_]?DD\[-_]?(HH[-_:]?MM[-_:]?SS)?`.
///
/// Only years in the range 1900 to 2100 are accepted. If the digits following the date do not
/// form a valid time, only the date is used.
pub struct NaiveFileNameParser {}

impl NaiveFileNameParser {
    /// Extracts the date and time from the matched groups.
    ///
    /// The returned flag is `true` if the capture contains digits for a time which were ignored
    /// because they do not form a valid time. In that case the time is set to 00:00.
    fn parse_capture(capture: &Captures) -> anyhow::Result<Option<(NaiveDateTime, bool)>> {
        let year = capture
            .name("year")
            .ok_or_else(|| anyhow!("Regex did not find year group"))?
            .as_str()
            .parse::<i32>()?;
        let month = capture
            .name("month")
            .ok_or_else(|| anyhow!("Regex did not find month group"))?
            .as_str()
            .parse::<u32>()?;
        let day = capture
            .name("day")
            .ok_or_else(|| anyhow!("Regex did not find day group"))?
            .as_str()
            .parse::<u32>()?;

        if !PLAUSIBLE_YEARS.contains(&year) {
            // most likely not a date, but some other number
            return Ok(None);
        }

        let date =
            NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| anyhow!("Invalid date"))?;

        let (time, time_ignored) = if let (Some(hour), Some(minute), Some(second)) = (
            capture.name("hour"),
            capture.name("minute"),
            capture.name("second"),
        ) {
            match NaiveTime::from_hms_opt(
                hour.as_str().parse::<u32>()?,
                minute.as_str().parse::<u32>()?,
                second.as_str().parse::<u32>()?,
            ) {
                Some(time) => (time, false),
                // the digits after the date are not a time, keep the date and assume 00:00
                None => (NaiveTime::MIN, true),
            }
        } else {
            // no time given, assume 00:00
            (NaiveTime::MIN, false)
        };

        Ok(Some((NaiveDateTime::new(date, time), time_ignored)))
    }
}

impl FileNameToDateTransformer for NaiveFileNameParser {
    fn get_regex(&self) -> &Regex {
        &RE_NAIVE_FILENAME
    }

    fn transform(&self, capture: &Captures) -> anyhow::Result<Option<NaiveDateTime>> {
        Ok(Self::parse_capture(capture)?.map(|(date_time, _)| date_time))
    }

    fn try_transform_name(&self, name: &str) -> anyhow::Result<Option<(NaiveDateTime, String)>> {
        for capture in self.get_regex().captures_iter(name) {
            match Self::parse_capture(&capture) {
                Ok(Some((date_time, time_ignored))) => {
                    // Only remove the part of the name that was interpreted: if the digits
                    // after the date were not used as time, they stay part of the name.
                    let interpreted = if time_ignored {
                        capture.name("date")
                    } else {
                        capture.get(0)
                    };
                    let remaining_name = match interpreted {
                        Some(interpreted) => {
                            format!(
                                "{}{}",
                                &name[..interpreted.start()],
                                &name[interpreted.end()..]
                            )
                        }
                        None => name.to_string(),
                    };
                    return Ok(Some((date_time, remaining_name)));
                }
                Ok(None) => {}
                Err(e) => {
                    log::error!("Error: {e:?}");
                }
            }
        }
        Ok(None) // No match found
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
