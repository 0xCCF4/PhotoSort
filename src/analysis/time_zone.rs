use anyhow::anyhow;
use chrono::{DateTime, FixedOffset, Local, MappedLocalTime, NaiveDateTime, TimeZone as _, Utc};
use std::fmt;
use std::str::FromStr;

/// A date and time with a UTC offset. The offset comes from the file, or from the ingest time zone.
pub type ZonedDateTime = DateTime<FixedOffset>;

/// A time zone, as a fixed offset from UTC.
///
/// The option `--time-zone` uses this type for a value that carries no offset of its own.
/// The option `--output-time-zone` uses it for the zone that the file names show.
///
/// An offset that a file states always wins. A photo states its offset in the `OffsetTime` fields of
/// EXIF. A video states it in the creation date of an Apple device, and an Android device writes
/// UTC. Only a value without such an offset is read in the given zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeZone {
    /// The local time of this computer.
    #[default]
    Local,
    /// UTC.
    Utc,
    /// A fixed offset from UTC, for example `+02:00`.
    Fixed(FixedOffset),
}

impl FromStr for TimeZone {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" | "l" => Ok(TimeZone::Local),
            "utc" | "z" => Ok(TimeZone::Utc),
            _ => parse_offset(s).map(TimeZone::Fixed).ok_or_else(|| {
                anyhow!(
                    "Invalid time zone: {s}. Possible values are local, utc or a fixed offset such as +02:00"
                )
            }),
        }
    }
}

impl fmt::Display for TimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeZone::Local => write!(f, "local"),
            TimeZone::Utc => write!(f, "utc"),
            TimeZone::Fixed(offset) => write!(f, "{offset}"),
        }
    }
}

impl TimeZone {
    /// Reads a bare wall clock value in this time zone.
    #[must_use]
    pub fn read(self, time: NaiveDateTime) -> ZonedDateTime {
        match self {
            TimeZone::Utc => Utc.from_utc_datetime(&time).fixed_offset(),
            TimeZone::Local => local_time(time),
            TimeZone::Fixed(offset) => read_in_offset(time, offset),
        }
    }

    /// Converts a date and time to this time zone.
    #[must_use]
    pub fn apply(self, time: ZonedDateTime) -> ZonedDateTime {
        match self {
            TimeZone::Local => time.with_timezone(&Local).fixed_offset(),
            TimeZone::Utc => time.with_timezone(&Utc).fixed_offset(),
            TimeZone::Fixed(offset) => time.with_timezone(&offset),
        }
    }
}

/// Reads a wall clock value as the local time of this computer.
///
/// A local time that occurs twice during the move back from daylight saving time takes the offset
/// before the change. A local time inside the gap of the move forward does not exist. It takes the
/// offset that is in effect at that value read as UTC.
fn local_time(time: NaiveDateTime) -> ZonedDateTime {
    match Local.from_local_datetime(&time) {
        MappedLocalTime::Single(local) | MappedLocalTime::Ambiguous(local, _) => {
            local.fixed_offset()
        }
        MappedLocalTime::None => {
            let offset = Local.offset_from_utc_datetime(&time);
            ZonedDateTime::from_naive_utc_and_offset(time, offset)
        }
    }
}

/// Reads a wall clock value in the given offset.
pub(crate) fn read_in_offset(time: NaiveDateTime, offset: FixedOffset) -> ZonedDateTime {
    ZonedDateTime::from_naive_utc_and_offset(time - offset, offset)
}

/// Parses an offset such as `+02:00`, `+0200`, `-05` or `Z` into a [`FixedOffset`].
pub(crate) fn parse_offset(value: &str) -> Option<FixedOffset> {
    if value.eq_ignore_ascii_case("z") {
        return FixedOffset::east_opt(0);
    }
    let sign = match value.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let rest = value.get(1..)?;
    let (hours, minutes) = match rest.split_once(':') {
        Some((hours, minutes)) => (hours, minutes),
        None => match rest.len() {
            2 => (rest, "0"),
            4 => rest.split_at(2),
            _ => return None,
        },
    };
    let hours: i32 = hours.parse().ok()?;
    let minutes: i32 = minutes.parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
}
