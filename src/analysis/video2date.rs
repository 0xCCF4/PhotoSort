use anyhow::anyhow;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use ffmpeg_next as ffmpeg;
use log::trace;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;

static FFMPEG_INITIALIZED: Mutex<bool> = Mutex::new(false);

/// Metadata key that Apple devices write. It holds the local time of the recording including its
/// UTC offset, for example `2023-06-16T00:30:00+0200`.
const APPLE_CREATION_DATE_KEY: &str = "com.apple.quicktime.creationdate";
/// Prefix of the metadata keys that Android devices write. The prefix marks a file that an Android
/// device created, and Android writes UTC into the `creation_time` field.
const ANDROID_METADATA_KEY_PREFIX: &str = "com.android.";
/// Metadata key of the container timestamp.
const CREATION_TIME_KEY: &str = "creation_time";

fn init_ffmpeg() -> anyhow::Result<()> {
    match FFMPEG_INITIALIZED.lock() {
        Ok(mut guard) => {
            if *guard {
                return Ok(());
            }
            ffmpeg::init().map_err(|e| anyhow!("Error initializing ffmpeg: {e:?}"))?;
            *guard = true;
            Ok(())
        }
        Err(poisoned) => Err(anyhow!("Mutex poisoned: {poisoned:?}")),
    }
}

/// How the creation time of a video is interpreted.
///
/// The `creation_time` field of a video container holds a bare wall clock value without a time
/// zone, and devices disagree about its meaning. Apple devices write UTC and add the local time of
/// the recording with its UTC offset under the key `com.apple.quicktime.creationdate`. Android
/// devices write UTC. Many cameras write the local time of the recording, without any comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoTimeZone {
    /// Use the time zone information that the file carries. When the file has the Apple creation
    /// date, that local time is used. When the file comes from an Android device, its UTC value is
    /// converted to the local time of this computer. All other files are used as written.
    #[default]
    Auto,
    /// Use the value as written. This is the local time of the recording for most cameras.
    Local,
    /// Treat the value as UTC and convert it to the local time of this computer.
    Utc,
    /// Treat the value as UTC and convert it to the given offset. Useful when this computer is not
    /// in the time zone of the recording.
    Fixed(FixedOffset),
}

impl FromStr for VideoTimeZone {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" | "a" => Ok(VideoTimeZone::Auto),
            "local" | "l" => Ok(VideoTimeZone::Local),
            "utc" | "z" => Ok(VideoTimeZone::Utc),
            _ => parse_offset(s).map(VideoTimeZone::Fixed).ok_or_else(|| {
                anyhow!(
                    "Invalid video time zone: {s}. Possible values are auto, local, utc or a fixed offset such as +02:00"
                )
            }),
        }
    }
}

impl fmt::Display for VideoTimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoTimeZone::Auto => write!(f, "auto"),
            VideoTimeZone::Local => write!(f, "local"),
            VideoTimeZone::Utc => write!(f, "utc"),
            VideoTimeZone::Fixed(offset) => write!(f, "{offset}"),
        }
    }
}

/// Parses an offset such as `+02:00`, `+0200` or `-05` into a [`FixedOffset`].
fn parse_offset(value: &str) -> Option<FixedOffset> {
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

/// Returns whether the metadata carries the keys that Android devices write.
fn from_android(metadata: ffmpeg::DictionaryRef<'_>) -> bool {
    metadata
        .iter()
        .any(|(key, _)| key.starts_with(ANDROID_METADATA_KEY_PREFIX))
}

impl VideoTimeZone {
    /// Resolves [`VideoTimeZone::Auto`] for a file. `from_android` states whether the file comes
    /// from an Android device. An explicit zone is returned unchanged.
    #[must_use]
    pub fn resolve(self, from_android: bool) -> Self {
        match self {
            VideoTimeZone::Auto if from_android => VideoTimeZone::Utc,
            VideoTimeZone::Auto => VideoTimeZone::Local,
            zone => zone,
        }
    }
}

/// Applies the interpretation of a container timestamp. The value is a UTC time when the zone is
/// `utc` or a fixed offset, and a local time otherwise.
fn apply_zone(time: NaiveDateTime, zone: VideoTimeZone) -> NaiveDateTime {
    let utc = Utc.from_utc_datetime(&time);
    match zone {
        VideoTimeZone::Utc => utc.with_timezone(&Local).naive_local(),
        VideoTimeZone::Fixed(offset) => utc.with_timezone(&offset).naive_local(),
        VideoTimeZone::Auto | VideoTimeZone::Local => time,
    }
}

/// Parses a date that contains its UTC offset, for example `2023-06-16T00:30:00+0200`, and returns
/// its local time. A date without an offset is returned as it is written.
fn parse_local_date(value: &str) -> Option<NaiveDateTime> {
    // `%+` is ISO 8601 / RFC 3339. It accepts the offset with and without a colon, and `Z`.
    DateTime::parse_from_str(value, "%+")
        .map(|date_time| date_time.naive_local())
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f").ok())
}

/// This function retrieves the date and time from the video metadata. The function uses the
/// `ffmpeg` crate to read the metadata from the video file.
///
/// The creation time is interpreted as [`VideoTimeZone::Auto`] describes it.
///
/// # Arguments
/// * `path` - A reference to a `Path` object.
///
/// # Returns
/// * `Some(NaiveDateTime)` - If the date and time could be retrieved from the video metadata.
/// * `None` - If there is no date and time in the video metadata.
///
/// # Errors
/// This function will return an error if:
/// * The video file could not be read.
pub fn get_video_time<P: AsRef<Path>>(path: P) -> anyhow::Result<Option<NaiveDateTime>> {
    get_video_time_with_zone(path, VideoTimeZone::default())
}

/// This function retrieves the date and time from the video metadata, like
/// [`get_video_time`], and interprets the value as `zone` describes it.
///
/// # Arguments
/// * `path` - A reference to a `Path` object.
/// * `zone` - How to interpret the creation time of the file. See [`VideoTimeZone`] for details.
///
/// # Returns
/// * `Some(NaiveDateTime)` - If the date and time could be retrieved from the video metadata.
/// * `None` - If there is no date and time in the video metadata.
///
/// # Errors
/// This function will return an error if:
/// * The video file could not be read.
/// * The creation time cannot be parsed.
pub fn get_video_time_with_zone<P: AsRef<Path>>(
    path: P,
    zone: VideoTimeZone,
) -> anyhow::Result<Option<NaiveDateTime>> {
    init_ffmpeg()?;

    let instance = ffmpeg::format::input(&path)?;
    let metadata = instance.metadata();

    // The Apple creation date is unambiguous, it holds the local time with its UTC offset.
    if let Some(value) = metadata.get(APPLE_CREATION_DATE_KEY) {
        trace!(
            "Found {APPLE_CREATION_DATE_KEY} ({value}) for video: {:?}",
            path.as_ref()
        );
        if let Some(local_time) = parse_local_date(value) {
            return Ok(Some(local_time));
        }
        trace!("Could not parse {APPLE_CREATION_DATE_KEY}, falling back to {CREATION_TIME_KEY}");
    }

    let Some(value) = metadata.get(CREATION_TIME_KEY) else {
        return Ok(None);
    };
    trace!(
        "Found {CREATION_TIME_KEY} ({value}) for video: {:?}",
        path.as_ref()
    );
    // ffmpeg appends a `Z` to the value whatever the device wrote, so the offset carries no
    // information. Parse the digits and let the video time zone decide how to read them.
    let time = NaiveDateTime::parse_from_str(value, "%+").map_err(|e| {
        anyhow!(
            "Could not parse {CREATION_TIME_KEY} ({value}) of {:?}: {e}",
            path.as_ref()
        )
    })?;

    let zone = zone.resolve(from_android(metadata));
    trace!(
        "Interpreting {CREATION_TIME_KEY} of {:?} as {zone}",
        path.as_ref()
    );

    Ok(Some(apply_zone(time, zone)))
}
