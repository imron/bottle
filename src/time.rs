use std::fmt;

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::fmt::strtime;
use jiff::tz::TimeZone;

use crate::error::{Error, Fail, Usage};
use crate::spec::TimePeriod;

/// Unix seconds. Sub-second time is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(Timestamp);

impl Instant {
    pub fn now() -> Self {
        Self::from_timestamp(Timestamp::now())
    }

    pub fn from_timestamp(ts: Timestamp) -> Self {
        Self(seconds_only(ts))
    }

    pub fn timestamp(self) -> Timestamp {
        self.0
    }
}

fn seconds_only(ts: Timestamp) -> Timestamp {
    // as_second() of a Timestamp is always in range for from_second
    // (jiff semver). Keep ts rather than invent another instant.
    Timestamp::from_second(ts.as_second()).unwrap_or(ts)
}

pub fn zone(name: Option<&str>) -> Result<TimeZone, Error> {
    match name {
        None => Ok(TimeZone::system()),
        Some(name) => Ok(TimeZone::get(name)?),
    }
}

pub fn parse_instant(input: &str, tz: &TimeZone) -> Result<Instant, Error> {
    match parse(input, tz)? {
        Parsed::Instant(ts) => Ok(Instant::from_timestamp(ts)),
        Parsed::Date(_) => Err(Error::Usage(Usage::DateOnlyNotInstant)),
    }
}

pub fn display_local(at: Instant, tz: &TimeZone) -> Result<String, Error> {
    let zoned = at.timestamp().to_zoned(tz.clone());
    Ok(strtime::format("%Y-%m-%dT%H:%M:%S%:z", &zoned)?)
}

fn from_bound(input: &str, tz: &TimeZone) -> Result<Instant, Error> {
    match parse(input, tz)? {
        Parsed::Instant(ts) => Ok(Instant::from_timestamp(ts)),
        Parsed::Date(date) => Ok(Instant::from_timestamp(date_midnight(date, tz)?)),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ToBound {
    Inclusive(Instant),
    Exclusive(Instant),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Range {
    pub from: Option<Instant>,
    pub to: Option<ToBound>,
}

impl Range {
    pub fn parse(from: Option<&str>, to: Option<&str>, tz: &TimeZone) -> Result<Self, Error> {
        Ok(Self {
            from: from.map(|s| from_bound(s, tz)).transpose()?,
            to: to.map(|s| to_bound(s, tz)).transpose()?,
        })
    }

    pub fn today(tz: &TimeZone) -> Result<Self, Error> {
        let today = Timestamp::now().to_zoned(tz.clone()).date();
        let start = Instant::from_timestamp(date_midnight(today, tz)?);
        let next = today.checked_add(jiff::Span::new().days(1))?;
        let end = Instant::from_timestamp(date_midnight(next, tz)?);
        Ok(Self {
            from: Some(start),
            to: Some(ToBound::Exclusive(end)),
        })
    }
}

fn to_bound(input: &str, tz: &TimeZone) -> Result<ToBound, Error> {
    match parse(input, tz)? {
        Parsed::Instant(ts) => Ok(ToBound::Inclusive(Instant::from_timestamp(ts))),
        Parsed::Date(date) => {
            let next = date.checked_add(jiff::Span::new().days(1))?;
            Ok(ToBound::Exclusive(Instant::from_timestamp(date_midnight(
                next, tz,
            )?)))
        }
    }
}

pub fn local_civil(at: Instant, tz: &TimeZone) -> Date {
    at.timestamp().to_zoned(tz.clone()).date()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Period {
    Day(Date),
    Week { year: i16, week: i8 },
    Month { year: i16, month: i8 },
    Year(i16),
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Day(date) => write!(f, "{date}"),
            Self::Week { year, week } => write!(f, "{year}-W{week:02}"),
            Self::Month { year, month } => write!(f, "{year:04}-{month:02}"),
            Self::Year(year) => write!(f, "{year:04}"),
        }
    }
}

impl Period {
    pub fn parse(unit: TimePeriod, raw: &str) -> Result<Self, Error> {
        let bad = || Error::Fail(Fail::Store(format!("corrupt period: {raw}")));
        match unit {
            TimePeriod::Day => {
                let date: Date = raw.parse().map_err(|_| bad())?;
                Ok(Self::Day(date))
            }
            TimePeriod::Week => {
                let (year, week) = raw.rsplit_once("-W").ok_or_else(bad)?;
                Ok(Self::Week {
                    year: year.parse().map_err(|_| bad())?,
                    week: week.parse().map_err(|_| bad())?,
                })
            }
            TimePeriod::Month => {
                let (year, month) = raw.rsplit_once('-').ok_or_else(bad)?;
                Ok(Self::Month {
                    year: year.parse().map_err(|_| bad())?,
                    month: month.parse().map_err(|_| bad())?,
                })
            }
            TimePeriod::Year => Ok(Self::Year(raw.parse().map_err(|_| bad())?)),
        }
    }
}

pub fn period(unit: TimePeriod, at: Instant, tz: &TimeZone) -> Period {
    let date = local_civil(at, tz);
    match unit {
        TimePeriod::Day => Period::Day(date),
        TimePeriod::Month => Period::Month {
            year: date.year(),
            month: date.month(),
        },
        TimePeriod::Year => Period::Year(date.year()),
        TimePeriod::Week => {
            let iso = date.iso_week_date();
            Period::Week {
                year: iso.year(),
                week: iso.week(),
            }
        }
    }
}

enum Parsed {
    Instant(Timestamp),
    Date(Date),
}

fn parse(input: &str, tz: &TimeZone) -> Result<Parsed, Error> {
    if input.contains(' ') {
        return Err(Error::Usage(Usage::TimeMustUseT));
    }
    if looks_like_date(input) {
        let date: Date = input
            .parse()
            .map_err(|_| Error::Usage(Usage::InvalidDate(input.to_string())))?;
        return Ok(Parsed::Date(date));
    }
    let Some((date, rest)) = input.split_once('T') else {
        return Err(Error::Usage(Usage::InvalidTime(input.to_string())));
    };
    if !looks_like_date(date) || !looks_like_hms(rest.get(..8).unwrap_or("")) {
        return Err(Error::Usage(Usage::InvalidTime(input.to_string())));
    }
    if rest.len() == 8 {
        let dt: DateTime = strtime::parse("%Y-%m-%dT%H:%M:%S", input)
            .and_then(|p| p.to_datetime())
            .map_err(|_| Error::Usage(Usage::InvalidTime(input.to_string())))?;
        let zoned = dt
            .to_zoned(tz.clone())
            .map_err(|e| Error::Usage(Usage::InvalidTime(e.to_string())))?;
        return Ok(Parsed::Instant(zoned.timestamp()));
    }
    if rest.ends_with('Z') {
        if rest.len() != 9 {
            return Err(Error::Usage(Usage::InvalidTime(input.to_string())));
        }
        let ts: Timestamp = input
            .parse()
            .map_err(|_| Error::Usage(Usage::InvalidTime(input.to_string())))?;
        return Ok(Parsed::Instant(ts));
    }
    let Some(sign_at) = rest.rfind(['+', '-']) else {
        return Err(Error::Usage(Usage::InvalidTime(input.to_string())));
    };
    if sign_at != 8 {
        return Err(Error::Usage(Usage::InvalidTime(input.to_string())));
    }
    let offset = &rest[sign_at..];
    if offset.len() != 6 || offset.as_bytes().get(3) != Some(&b':') {
        return Err(Error::Usage(Usage::OffsetNeedsColon));
    }
    let zoned = strtime::parse("%Y-%m-%dT%H:%M:%S%:z", input)
        .and_then(|p| p.to_zoned())
        .map_err(|_| Error::Usage(Usage::InvalidTime(input.to_string())))?;
    Ok(Parsed::Instant(zoned.timestamp()))
}

fn looks_like_date(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes()[4] == b'-'
        && s.as_bytes()[7] == b'-'
        && s.bytes().all(|b| b == b'-' || b.is_ascii_digit())
}

fn looks_like_hms(s: &str) -> bool {
    s.len() == 8
        && s.as_bytes()[2] == b':'
        && s.as_bytes()[5] == b':'
        && s.bytes().all(|b| b == b':' || b.is_ascii_digit())
}

fn date_midnight(date: Date, tz: &TimeZone) -> Result<Timestamp, Error> {
    Ok(date.to_zoned(tz.clone())?.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn melbourne() -> TimeZone {
        TimeZone::get("Australia/Melbourne").unwrap()
    }

    #[test]
    fn non_ascii_time_tail_is_invalid_not_panic() {
        let tz = TimeZone::UTC;
        for input in ["2026-08-22T08:14:0µ", "2026-08-22T08:14:🎉"] {
            let err = parse_instant(input, &tz).unwrap_err();
            assert!(
                matches!(err, Error::Usage(Usage::InvalidTime(ref s)) if s == input),
                "{input}: {err}"
            );
            let err = Range::parse(Some(input), None, &tz).unwrap_err();
            assert!(
                matches!(err, Error::Usage(Usage::InvalidTime(ref s)) if s == input),
                "{input}: {err}"
            );
        }
    }

    #[test]
    fn period_display_round_trips() {
        let tz = melbourne();
        let at = parse_instant("2026-08-22T08:14:00+10:00", &tz).unwrap();
        for unit in [
            TimePeriod::Day,
            TimePeriod::Week,
            TimePeriod::Month,
            TimePeriod::Year,
        ] {
            let p = period(unit, at, &tz);
            assert_eq!(Period::parse(unit, &p.to_string()).unwrap(), p, "{unit:?}");
        }
        assert_eq!(period(TimePeriod::Day, at, &tz).to_string(), "2026-08-22");
        assert_eq!(period(TimePeriod::Month, at, &tz).to_string(), "2026-08");
        assert_eq!(period(TimePeriod::Year, at, &tz).to_string(), "2026");
    }

    #[test]
    fn instants_are_whole_seconds() {
        let ts = Timestamp::new(1_777_000_000, 123_456_789).unwrap();
        let at = Instant::from_timestamp(ts);
        assert_eq!(
            at.timestamp(),
            Timestamp::from_second(1_777_000_000).unwrap()
        );
        let now = Instant::now();
        assert_eq!(
            now.timestamp(),
            Timestamp::from_second(now.timestamp().as_second()).unwrap()
        );
    }

    #[test]
    fn instant_does_not_become_epoch() {
        for ts in [Timestamp::MIN, Timestamp::MAX] {
            let at = Instant::from_timestamp(ts);
            assert_eq!(
                at.timestamp(),
                Timestamp::from_second(ts.as_second()).unwrap()
            );
            assert_ne!(at.timestamp(), Timestamp::UNIX_EPOCH);
        }
    }

    #[test]
    fn z_offset_and_naive_local_are_one_instant() {
        let tz = melbourne();
        let z = parse_instant("2026-08-21T22:14:00Z", &tz).unwrap();
        let offset = parse_instant("2026-08-22T08:14:00+10:00", &tz).unwrap();
        let naive = parse_instant("2026-08-22T08:14:00", &tz).unwrap();
        assert_eq!(z, offset);
        assert_eq!(z, naive);
        assert_eq!(display_local(z, &tz).unwrap(), "2026-08-22T08:14:00+10:00");
    }

    #[test]
    fn negative_offset_is_an_instant() {
        let tz = melbourne();
        let at = parse_instant("2026-08-22T08:14:00-05:00", &tz).unwrap();
        let utc = parse_instant("2026-08-22T13:14:00Z", &tz).unwrap();
        assert_eq!(at, utc);
    }

    #[test]
    fn date_only_is_a_query_bound() {
        let tz = melbourne();
        assert!(matches!(
            parse_instant("2026-08-22", &tz).unwrap_err(),
            Error::Usage(Usage::DateOnlyNotInstant)
        ));
        let range = Range::parse(Some("2026-08-22"), Some("2026-08-22"), &tz).unwrap();
        let from = range.from.unwrap();
        let Some(ToBound::Exclusive(to)) = range.to else {
            panic!("{:?}", range.to);
        };
        assert_eq!(
            display_local(from, &tz).unwrap(),
            "2026-08-22T00:00:00+10:00"
        );
        assert_eq!(display_local(to, &tz).unwrap(), "2026-08-23T00:00:00+10:00");
    }

    #[test]
    fn dst_civil_days_are_23_or_25_hours() {
        let tz = melbourne();
        let hours = |day: &str| -> i64 {
            let range = Range::parse(Some(day), Some(day), &tz).unwrap();
            let from = range.from.unwrap();
            let Some(ToBound::Exclusive(to)) = range.to else {
                panic!("{day}: {:?}", range.to);
            };
            (to.timestamp().as_second() - from.timestamp().as_second()) / 3600
        };
        assert_eq!(hours("2026-10-04"), 23);
        assert_eq!(hours("2026-04-05"), 25);
        assert_eq!(hours("2026-08-22"), 24);
    }

    #[test]
    fn dst_gap_naive_time_is_the_later_instant() {
        let tz = melbourne();
        let gap = parse_instant("2026-10-04T02:30:00", &tz).unwrap();
        let later = parse_instant("2026-10-04T03:30:00", &tz).unwrap();
        assert_eq!(gap, later);
        assert_eq!(
            display_local(gap, &tz).unwrap(),
            "2026-10-04T03:30:00+11:00"
        );
    }

    #[test]
    fn rejects_bad_time_and_date_shapes() {
        let tz = melbourne();
        for input in [
            "2026-08-22T08:14",
            "2026-08-22t08:14:00",
            "2026-08-22T08:14:00z",
            "2026-08-22T25:00:00",
            "2026-08-22T08:61:00",
        ] {
            let err = parse_instant(input, &tz).unwrap_err();
            assert!(
                matches!(err, Error::Usage(Usage::InvalidTime(ref s)) if s == input),
                "{input}: {err}"
            );
        }
        for input in ["2026-02-30", "2025-02-29"] {
            let err = parse_instant(input, &tz).unwrap_err();
            assert!(
                matches!(err, Error::Usage(Usage::InvalidDate(ref s)) if s == input),
                "{input}: {err}"
            );
        }
    }
}
