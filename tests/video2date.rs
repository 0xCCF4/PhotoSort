//! Tests for the video date analysis and the interpretation of its time zone.
#![cfg(feature = "video")]

use chrono::{Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone as _, Utc};
use photo_sort::analysis::time_zone::{TimeZone, ZonedDateTime};
use photo_sort::analysis::video2date::get_video_time;
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

/// Returns the instant of a zoned value as a naive UTC value.
fn utc(time: ZonedDateTime) -> NaiveDateTime {
    time.with_timezone(&Utc).naive_utc()
}

/// Returns the naive UTC value that a local time on this computer denotes.
fn as_utc(time: NaiveDateTime) -> NaiveDateTime {
    Local
        .from_local_datetime(&time)
        .earliest()
        .expect("valid local time")
        .with_timezone(&Utc)
        .naive_utc()
}

/// Returns a fixed offset in hours.
fn fixed(hours: i32) -> FixedOffset {
    FixedOffset::east_opt(hours * 3600).expect("valid offset")
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
    let result = get_video_time(&path, TimeZone::Local)
        .expect("Failed to read video")
        .expect("date in the metadata");
    assert_eq!(utc(result), time(22));
    assert_eq!(
        *result.offset(),
        FixedOffset::east_opt(2 * 3600).expect("valid offset")
    );
    // The creation date is unambiguous, so it wins over every ingest zone.
    for zone in [TimeZone::Local, TimeZone::Utc, TimeZone::Fixed(fixed(2))] {
        let result = get_video_time(&path, zone)
            .expect("Failed to read video")
            .expect("date in the metadata");
        assert_eq!(utc(result), time(22), "zone {zone}");
    }
}

#[test]
fn parses_the_creation_date_of_apple_devices() {
    let dir = TempDir::new("video_apple_forms");
    // The local time of the creation date in the test values without an offset.
    let expected_local = NaiveDate::from_ymd_opt(2023, 6, 16)
        .expect("valid date")
        .and_hms_opt(0, 30, 0)
        .expect("valid time");
    let cases = [
        ("2023-06-16T00:30:00+0200", Some(time(22))),
        ("2023-06-16T00:30:00+02:00", Some(time(22))),
        ("2023-06-15T22:30:00Z", Some(time(22))),
        ("2023-06-16T00:30:00", Some(as_utc(expected_local))),
        (
            "2023-06-16T00:30:00.123456789+0200",
            Some(time(22) + Duration::nanoseconds(123_456_789)),
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
        let result = get_video_time(&path, TimeZone::Local)
            .expect("Failed to read video")
            .expect("date in the metadata");
        match expected {
            Some(expected) => assert_eq!(utc(result), *expected, "value {value}"),
            // A value without a date falls back to the container time.
            None => assert_eq!(utc(result), as_utc(time(22)), "value {value}"),
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
    // The ingest zone does not matter, an Android device writes UTC.
    for zone in [TimeZone::Local, TimeZone::Utc, TimeZone::Fixed(fixed(5))] {
        let result = get_video_time(&path, zone)
            .expect("Failed to read video")
            .expect("date in the metadata");
        assert_eq!(utc(result), time(22), "zone {zone}");
        assert_eq!(*result.offset(), fixed(0), "zone {zone}");
    }
}

#[test]
fn reads_the_written_time_of_cameras_in_the_ingest_zone() {
    let dir = TempDir::new("video_camera");
    let path = write_video(&dir, "camera.mp4", time(18), &[]);
    assert_eq!(
        utc(get_video_time(&path, TimeZone::Local)
            .expect("Failed to read video")
            .expect("date in the metadata")),
        as_utc(time(18))
    );
    assert_eq!(
        utc(get_video_time(&path, TimeZone::Utc)
            .expect("Failed to read video")
            .expect("date in the metadata")),
        time(18)
    );
    assert_eq!(
        utc(get_video_time(&path, TimeZone::Fixed(fixed(2)))
            .expect("Failed to read video")
            .expect("date in the metadata")),
        time(18) - Duration::hours(2)
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
    assert_eq!(
        get_video_time(&path, TimeZone::Local).expect("Failed to read video"),
        None
    );
}
