//! Tests for the ingest and output time zone settings.

use chrono::{FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone as _, Utc};
use photo_sort::analysis::time_zone::TimeZone;

/// Returns a fixed offset in hours.
fn fixed(hours: i32) -> FixedOffset {
    FixedOffset::east_opt(hours * 3600).expect("valid offset")
}

/// Returns the bare wall clock value that the tests use.
fn bare_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2025, 3, 1)
        .expect("valid date")
        .and_hms_opt(17, 10, 12)
        .expect("valid time")
}

#[test]
fn parses_time_zones() {
    let parse = |value: &str| value.parse::<TimeZone>().expect("valid time zone");
    assert_eq!(parse("local"), TimeZone::Local);
    assert_eq!(parse("LOCAL"), TimeZone::Local);
    assert_eq!(parse("utc"), TimeZone::Utc);
    assert_eq!(parse("+02:00"), TimeZone::Fixed(fixed(2)));
    assert_eq!(parse("+0200"), TimeZone::Fixed(fixed(2)));
    assert_eq!(parse("-05"), TimeZone::Fixed(fixed(-5)));
    assert_eq!(parse("+00:00"), TimeZone::Fixed(fixed(0)));
    assert_eq!(TimeZone::Fixed(fixed(2)).to_string(), "+02:00");
    for invalid in ["auto", "yesterday", "02:00", "+25:00", "+02:75", "+"] {
        assert!(
            invalid.parse::<TimeZone>().is_err(),
            "{invalid} should not parse as a time zone"
        );
    }
}

#[test]
fn reads_a_bare_time() {
    let time = bare_time();

    let utc = TimeZone::Utc.read(time);
    assert_eq!(utc.naive_local(), time);
    assert_eq!(utc.naive_utc(), time);
    assert_eq!(*utc.offset(), fixed(0));

    let local = Local
        .from_local_datetime(&time)
        .earliest()
        .expect("valid local time");
    let read = TimeZone::Local.read(time);
    assert_eq!(read.naive_local(), time);
    assert_eq!(read.with_timezone(&Utc), local.with_timezone(&Utc));

    let read = TimeZone::Fixed(fixed(2)).read(time);
    assert_eq!(read.naive_local(), time);
    assert_eq!(*read.offset(), fixed(2));
    assert_eq!(
        read.with_timezone(&Utc).naive_utc(),
        time - chrono::TimeDelta::hours(2)
    );
}

#[test]
fn applies_the_output_time_zone() {
    // 17:10:12 at +01:00 is 16:10:12 UTC.
    let time = fixed(1)
        .with_ymd_and_hms(2025, 3, 1, 17, 10, 12)
        .single()
        .expect("valid time");
    let date = |hour| {
        NaiveDate::from_ymd_opt(2025, 3, 1)
            .expect("valid date")
            .and_hms_opt(hour, 10, 12)
            .expect("valid time")
    };

    assert_eq!(TimeZone::Utc.apply(time).naive_local(), date(16));
    assert_eq!(
        TimeZone::Fixed(fixed(2)).apply(time).naive_local(),
        date(18)
    );
    assert_eq!(
        TimeZone::Local.apply(time).with_timezone(&Utc),
        time.with_timezone(&Utc),
        "the instant does not change"
    );
}
