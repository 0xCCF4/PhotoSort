//! Tests for the EXIF date analysis and the interpretation of its time zone.
//!
//! The tests that read a photo need the photos in `tests/src`, which are not part
//! of the repository. They are ignored by default. Run them with
//! `cargo test -- --include-ignored` on a checkout that has the photos.

use chrono::{FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone as _, Utc};
use photo_sort::analysis::exif2date::{get_exif_time, ExifDateType};
use photo_sort::analysis::time_zone::{TimeZone, ZonedDateTime};
use std::fs::File;
use std::path::PathBuf;

/// Returns the path of a photo that the tests read.
fn photo(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/src")
        .join(name)
}

/// Returns the creation date of `DSC06619.JPG`. The photo states the offset +01:00.
fn creation_date() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2025, 3, 1)
        .expect("valid date")
        .and_hms_opt(17, 10, 12)
        .expect("valid time")
}

/// Returns the creation date of `edited.jpg`. The photo states no offset.
fn date_without_offset() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2011, 3, 13)
        .expect("valid date")
        .and_hms_opt(1, 5, 58)
        .expect("valid time")
}

/// Returns a fixed offset in hours.
fn fixed(hours: i32) -> FixedOffset {
    FixedOffset::east_opt(hours * 3600).expect("valid offset")
}

/// Reads the creation date of a photo with the given zone.
fn read(name: &str, zone: TimeZone) -> ZonedDateTime {
    let file = File::open(photo(name)).expect("Failed to open test photo");
    get_exif_time(file, ExifDateType::Creation, zone)
        .expect("Failed to read EXIF data")
        .expect("Test photo has a creation date")
}

#[test]
#[ignore = "needs the photos in tests/src"]
fn uses_the_exif_offset_in_every_zone() {
    for zone in [TimeZone::Local, TimeZone::Utc, TimeZone::Fixed(fixed(2))] {
        let date = read("DSC06619.JPG", zone);
        assert_eq!(date.naive_local(), creation_date(), "zone {zone}");
        assert_eq!(*date.offset(), fixed(1), "zone {zone}");
        assert_eq!(
            date.with_timezone(&Utc).naive_utc(),
            creation_date() - fixed(1),
            "zone {zone}"
        );
    }
}

#[test]
#[ignore = "needs the photos in tests/src"]
fn reads_a_value_without_an_offset_in_the_given_zone() {
    let date = read("edited.jpg", TimeZone::Local);
    assert_eq!(date.naive_local(), date_without_offset());
    let expected = Local
        .from_local_datetime(&date_without_offset())
        .earliest()
        .expect("valid local time");
    assert_eq!(date.with_timezone(&Utc), expected.with_timezone(&Utc));

    let date = read("edited.jpg", TimeZone::Utc);
    assert_eq!(*date.offset(), fixed(0));
    assert_eq!(date.naive_utc(), date_without_offset());

    let date = read("edited.jpg", TimeZone::Fixed(fixed(2)));
    assert_eq!(*date.offset(), fixed(2));
    assert_eq!(date.naive_local(), date_without_offset());
}
