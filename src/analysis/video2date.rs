use crate::analysis::time_zone::{TimeZone, ZonedDateTime};
use anyhow::anyhow;
use chrono::{DateTime, NaiveDateTime};
use ffmpeg_next as ffmpeg;
use log::trace;
use std::path::Path;
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

/// Returns whether the metadata carries the keys that Android devices write.
fn from_android(metadata: ffmpeg::DictionaryRef<'_>) -> bool {
    metadata
        .iter()
        .any(|(key, _)| key.starts_with(ANDROID_METADATA_KEY_PREFIX))
}

/// Parses a date that contains its UTC offset, for example `2023-06-16T00:30:00+0200`, and returns
/// it with that offset. A date without an offset is read in `zone`.
fn parse_creation_date(value: &str, zone: TimeZone) -> Option<ZonedDateTime> {
    // `%+` is ISO 8601 / RFC 3339. It accepts the offset with and without a colon, and `Z`.
    DateTime::parse_from_str(value, "%+")
        .ok()
        .map(|date_time| date_time.fixed_offset())
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|time| zone.read(time))
        })
}

/// This function retrieves the date and time from the video metadata.
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
pub fn get_video_time<P: AsRef<Path>>(
    path: P,
    zone: TimeZone,
) -> anyhow::Result<Option<ZonedDateTime>> {
    init_ffmpeg()?;

    let instance = ffmpeg::format::input(&path)?;
    let metadata = instance.metadata();

    // The Apple creation date is unambiguous, it holds the local time with its UTC offset.
    if let Some(value) = metadata.get(APPLE_CREATION_DATE_KEY) {
        trace!(
            "Found {APPLE_CREATION_DATE_KEY} ({value}) for video: {:?}",
            path.as_ref()
        );
        if let Some(local_time) = parse_creation_date(value, zone) {
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
    // information. Parse the digits and read them in the zone of the device.
    let time = NaiveDateTime::parse_from_str(value, "%+").map_err(|e| {
        anyhow!(
            "Could not parse {CREATION_TIME_KEY} ({value}) of {:?}: {e}",
            path.as_ref()
        )
    })?;

    // An Android device writes UTC. Any other device is read via local TZ offset
    let zone = if from_android(metadata) {
        TimeZone::Utc
    } else {
        zone
    };
    trace!(
        "Interpreting {CREATION_TIME_KEY} of {:?} as {zone}",
        path.as_ref()
    );

    Ok(Some(zone.read(time)))
}
