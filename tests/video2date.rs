//! Tests for the video date analysis and the interpretation of its time zone.
#![cfg(feature = "video")]

use chrono::{Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use photo_sort::analysis::video2date::{get_video_time, get_video_time_with_zone, VideoTimeZone};
use photo_sort::testutils::TempDir;
use std::fs;
use std::path::PathBuf;

/// Seconds between the epoch of the MPEG-4 time fields (1904-01-01) and the Unix epoch.
const MP4_EPOCH_OFFSET: i64 = 2_082_844_800;

/// Builds a box of the ISO base media file format.
fn be_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(payload.len() + 8);
    data.extend_from_slice(
        &u32::try_from(payload.len() + 8)
            .expect("box too large")
            .to_be_bytes(),
    );
    data.extend_from_slice(kind);
    data.extend_from_slice(payload);
    data
}

/// Builds a full box, a box with a version and flags field.
fn full_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut data = vec![0u8; 4];
    data.extend_from_slice(payload);
    be_box(kind, &data)
}

/// Builds a video with a container creation time and the given metadata keys.
fn mp4(creation_time: NaiveDateTime, keys: &[(&str, &str)]) -> Vec<u8> {
    let seconds = creation_time.and_utc().timestamp() + MP4_EPOCH_OFFSET;
    let seconds = u32::try_from(seconds).expect("time out of range");

    let ftyp = be_box(
        b"ftyp",
        &[b"mp42".as_slice(), &512u32.to_be_bytes(), b"mp42isom"].concat(),
    );

    let mut mvhd = Vec::new();
    mvhd.extend_from_slice(&0u32.to_be_bytes()); // version 0, flags
    mvhd.extend_from_slice(&seconds.to_be_bytes()); // creation time
    mvhd.extend_from_slice(&seconds.to_be_bytes()); // modification time
    mvhd.extend_from_slice(&1000u32.to_be_bytes()); // timescale
    mvhd.extend_from_slice(&0u32.to_be_bytes()); // duration
    mvhd.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // rate
    mvhd.extend_from_slice(&0x0100u16.to_be_bytes()); // volume
    mvhd.extend_from_slice(&[0u8; 10]); // reserved
    for value in [0x0001_0000u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
        mvhd.extend_from_slice(&value.to_be_bytes()); // matrix
    }
    mvhd.extend_from_slice(&[0u8; 24]); // predefined
    mvhd.extend_from_slice(&2u32.to_be_bytes()); // next track id
    let mvhd = be_box(b"mvhd", &mvhd);

    let mut hdlr = Vec::new();
    hdlr.extend_from_slice(&0u32.to_be_bytes()); // pre defined
    hdlr.extend_from_slice(b"mdta"); // handler type
    hdlr.extend_from_slice(&[0u8; 12]); // reserved
    let hdlr = full_box(b"hdlr", &hdlr);

    // The keys box declares the names, the ilst box refers to them by their index.
    let mut keys_payload = Vec::new();
    keys_payload.extend_from_slice(
        &u32::try_from(keys.len())
            .expect("too many keys")
            .to_be_bytes(),
    );
    let mut items = Vec::new();
    for (index, (key, value)) in keys.iter().enumerate() {
        let mut name = key.as_bytes().to_vec();
        name.push(0);
        keys_payload.extend_from_slice(
            &u32::try_from(name.len() + 8)
                .expect("key too long")
                .to_be_bytes(),
        );
        keys_payload.extend_from_slice(b"mdta");
        keys_payload.extend_from_slice(&name);

        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_be_bytes()); // well known type: UTF-8
        data.extend_from_slice(&0u32.to_be_bytes()); // country and language
        data.extend_from_slice(value.as_bytes());
        // The type of an item is the index of its key, the payload is the data box.
        items.extend_from_slice(&be_box(
            &u32::try_from(index + 1)
                .expect("too many keys")
                .to_be_bytes(),
            &be_box(b"data", &data),
        ));
    }
    let keys_box = full_box(b"keys", &keys_payload);
    let ilst = be_box(b"ilst", &items);

    let meta = full_box(b"meta", &[hdlr, keys_box, ilst].concat());
    let moov = be_box(b"moov", &[mvhd, meta].concat());

    [ftyp, moov].concat()
}

/// Writes a video with a container creation time and the given metadata keys.
fn write_video(
    dir: &TempDir,
    name: &str,
    creation_time: NaiveDateTime,
    keys: &[(&str, &str)],
) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, mp4(creation_time, keys)).expect("Failed to write test video");
    path
}

/// Returns a time on the day of the test files, 30 minutes after the given hour.
fn time(hour: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2023, 6, 15)
        .expect("valid date")
        .and_hms_opt(hour, 30, 0)
        .expect("valid time")
}

/// Returns the local time that a UTC value denotes on this computer.
fn as_local(time: NaiveDateTime) -> NaiveDateTime {
    Utc.from_utc_datetime(&time)
        .with_timezone(&Local)
        .naive_local()
}

fn offset(hours: i32) -> FixedOffset {
    FixedOffset::east_opt(hours * 3600).expect("valid offset")
}

#[test]
fn parses_time_zones() {
    let parse = |value: &str| value.parse::<VideoTimeZone>().expect("valid time zone");
    assert_eq!(parse("auto"), VideoTimeZone::Auto);
    assert_eq!(parse("LOCAL"), VideoTimeZone::Local);
    assert_eq!(parse("utc"), VideoTimeZone::Utc);
    assert_eq!(parse("+02:00"), VideoTimeZone::Fixed(offset(2)));
    assert_eq!(parse("+0200"), VideoTimeZone::Fixed(offset(2)));
    assert_eq!(parse("-05"), VideoTimeZone::Fixed(offset(-5)));
    assert_eq!(parse("+00:00"), VideoTimeZone::Fixed(offset(0)));
    for invalid in ["yesterday", "02:00", "+25:00", "+02:75", "+"] {
        assert!(
            invalid.parse::<VideoTimeZone>().is_err(),
            "{invalid} should not parse"
        );
    }
}

#[test]
fn resolves_auto() {
    assert_eq!(VideoTimeZone::Auto.resolve(true), VideoTimeZone::Utc);
    assert_eq!(VideoTimeZone::Auto.resolve(false), VideoTimeZone::Local);
    assert_eq!(VideoTimeZone::Local.resolve(true), VideoTimeZone::Local);
    assert_eq!(VideoTimeZone::Utc.resolve(false), VideoTimeZone::Utc);
    assert_eq!(
        VideoTimeZone::Fixed(offset(2)).resolve(true),
        VideoTimeZone::Fixed(offset(2))
    );
}

#[test]
fn uses_the_creation_date_of_apple_devices() {
    let dir = TempDir::new("video_apple");
    let path = write_video(
        &dir,
        "apple.mp4",
        time(22),
        &[(
            "com.apple.quicktime.creationdate",
            "2023-06-16T00:30:00+0200",
        )],
    );
    let expected = NaiveDate::from_ymd_opt(2023, 6, 16)
        .expect("valid date")
        .and_hms_opt(0, 30, 0)
        .expect("valid time");
    assert_eq!(
        get_video_time(&path).expect("Failed to read video"),
        Some(expected)
    );
    // The creation date is unambiguous, so it wins in every mode.
    for zone in [
        VideoTimeZone::Auto,
        VideoTimeZone::Local,
        VideoTimeZone::Utc,
        VideoTimeZone::Fixed(offset(-7)),
    ] {
        assert_eq!(
            get_video_time_with_zone(&path, zone).expect("Failed to read video"),
            Some(expected),
            "zone {zone}"
        );
    }
}

#[test]
fn parses_the_creation_date_of_apple_devices() {
    let dir = TempDir::new("video_apple_forms");
    let expected = NaiveDate::from_ymd_opt(2023, 6, 16)
        .expect("valid date")
        .and_hms_opt(0, 30, 0)
        .expect("valid time");
    let cases = [
        ("2023-06-16T00:30:00+0200", Some(expected)),
        ("2023-06-16T00:30:00+02:00", Some(expected)),
        ("2023-06-15T22:30:00Z", Some(time(22))),
        ("2023-06-16T00:30:00", Some(expected)),
        (
            "2023-06-16T00:30:00.123456789+0200",
            Some(expected + Duration::nanoseconds(123_456_789)),
        ),
        ("not a date", None),
    ];
    for (index, (value, expected)) in cases.iter().enumerate() {
        let path = write_video(
            &dir,
            &format!("apple_{index}.mp4"),
            time(22),
            &[("com.apple.quicktime.creationdate", value)],
        );
        let result = get_video_time(&path).expect("Failed to read video");
        match expected {
            Some(expected) => assert_eq!(result, Some(*expected), "value {value}"),
            // A value without a date falls back to the container time.
            None => assert_eq!(result, Some(time(22)), "value {value}"),
        }
    }
}

#[test]
fn treats_videos_of_android_devices_as_utc() {
    let dir = TempDir::new("video_android");
    let path = write_video(
        &dir,
        "pixel.mp4",
        time(22),
        &[
            ("com.android.manufacturer", "Google"),
            ("com.android.model", "Pixel 7"),
        ],
    );
    assert_eq!(
        get_video_time(&path).expect("Failed to read video"),
        Some(as_local(time(22)))
    );
    assert_eq!(
        get_video_time_with_zone(&path, VideoTimeZone::Local).expect("Failed to read video"),
        Some(time(22))
    );
    assert_eq!(
        get_video_time_with_zone(&path, VideoTimeZone::Fixed(offset(2)))
            .expect("Failed to read video"),
        Some(time(22) + Duration::hours(2))
    );
}

#[test]
fn keeps_the_written_time_of_cameras() {
    let dir = TempDir::new("video_camera");
    let path = write_video(&dir, "camera.mp4", time(18), &[]);
    assert_eq!(
        get_video_time(&path).expect("Failed to read video"),
        Some(time(18))
    );
    assert_eq!(
        get_video_time_with_zone(&path, VideoTimeZone::Fixed(offset(2)))
            .expect("Failed to read video"),
        Some(time(18) + Duration::hours(2))
    );
    assert_eq!(
        get_video_time_with_zone(&path, VideoTimeZone::Utc).expect("Failed to read video"),
        Some(as_local(time(18)))
    );
}

#[test]
fn returns_none_without_a_creation_time() {
    let dir = TempDir::new("video_without_time");
    let path = dir.path().join("empty.mp4");
    let ftyp = be_box(
        b"ftyp",
        &[b"mp42".as_slice(), &512u32.to_be_bytes(), b"mp42isom"].concat(),
    );
    fs::write(&path, [ftyp, be_box(b"moov", &[])].concat()).expect("Failed to write test video");
    assert_eq!(get_video_time(&path).expect("Failed to read video"), None);
}
