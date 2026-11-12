use anyhow::anyhow;
use chrono::{DateTime, NaiveDateTime};
use ffmpeg_next as ffmpeg;
use log::trace;
use std::path::Path;
use std::sync::Mutex;

static FFMPEG_INITIALIZED: Mutex<bool> = Mutex::new(false);

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

/// Metadata key written by Apple devices. It contains the local time of the recording
/// including its UTC offset, for example `2023-06-16T00:30:00+0200`.
const APPLE_CREATION_DATE_KEY: &str = "com.apple.quicktime.creationdate";

/// Parses a date with UTC offset and returns its local (wall clock) time.
fn parse_local_creation_date(value: &str) -> Option<NaiveDateTime> {
    ["%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M:%S%.f%z"]
        .iter()
        .find_map(|format| DateTime::parse_from_str(value, format).ok())
        .or_else(|| DateTime::parse_from_rfc3339(value).ok())
        .map(|date_time| date_time.naive_local())
}

/// This function retrieves the date and time from the video metadata.
/// The function uses the `ffmpeg` crate to read the metadata from the video file.
///
/// The generic `creation_time` of a video is stored in UTC, without information about the time
/// zone of the recording. If the video additionally contains the creation date written by Apple
/// devices, which includes the UTC offset, the local time of the recording is returned instead.
/// This matches the EXIF dates of photos, which are local times as well.
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
    init_ffmpeg()?;

    let instance = ffmpeg::format::input(&path)?;
    let metadata = instance.metadata();

    if let Some(local_creation_date) = metadata.get(APPLE_CREATION_DATE_KEY) {
        trace!(
            "Found {APPLE_CREATION_DATE_KEY} ({local_creation_date}) for video: {:?}",
            path.as_ref()
        );
        if let Some(date_time) = parse_local_creation_date(local_creation_date) {
            return Ok(Some(date_time));
        }
        trace!("Could not parse {APPLE_CREATION_DATE_KEY}, falling back to creation_time");
    }

    let result = metadata
        .get("creation_time")
        .inspect(|x| trace!("Found creation_time ({x}) for video: {:?}", path.as_ref()))
        .map(|v| NaiveDateTime::parse_from_str(v, "%+")); // ISO 8601 / RFC 3339 format

    Ok(result.transpose()?)
}
