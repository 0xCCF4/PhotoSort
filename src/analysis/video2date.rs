use anyhow::anyhow;
use chrono::NaiveDateTime;
use ffmpeg_next as ffmpeg;
use std::path::Path;
use std::sync::Mutex;

static FFMPEG_INITIALIZED: Mutex<bool> = Mutex::new(false);

fn init_ffmpeg() -> anyhow::Result<()> {
    match FFMPEG_INITIALIZED.lock() {
        Ok(mut guard) => {
            if *guard {
                return Ok(());
            }
            ffmpeg::init().map_err(|e| anyhow!("Error initializing ffmpeg: {:?}", e))?;
            *guard = true;
            Ok(())
        }
        Err(poisoned) => Err(anyhow!("Mutex poisoned: {:?}", poisoned)),
    }
}

/// This function retrieves the date and time from the video metadata.
/// The function uses the `ffmpeg` crate to read the metadata from the video file.
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

    let result = instance
        .metadata()
        .get("creation_time")
        .map(|v| NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S%Z"));

    Ok(result.transpose()?)
}
