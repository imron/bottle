use std::fmt;

use jiff::Timestamp;
use jiff::civil::{Date, Time};
use jiff::fmt::strtime;
use jiff::tz::{Offset, TimeZone};

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

    fn next_second(self) -> Result<Self, Error> {
        Ok(Self::from_timestamp(
            self.0.checked_add(jiff::Span::new().seconds(1))?,
        ))
    }
}

/// How coarse an `at` value is. Inferred from the input shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grain {
    Instant,
    Day,
    Month,
}

impl Grain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Instant => "instant",
            Self::Day => "day",
            Self::Month => "month",
        }
    }

    pub fn parse(s: &str) -> Result<Self, Error> {
        match s {
            "instant" => Ok(Self::Instant),
            "day" => Ok(Self::Day),
            "month" => Ok(Self::Month),
            _ => Err(Error::Fail(Fail::CorruptStoredGrain(s.to_string()))),
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Instant => 0,
            Self::Day => 1,
            Self::Month => 2,
        }
    }

    pub fn at_most(self) -> impl Iterator<Item = Self> {
        [Self::Instant, Self::Day, Self::Month]
            .into_iter()
            .filter(move |g| g.rank() <= self.rank())
    }

    /// Coarsest event grain that can sit in this sum group.
    pub fn for_group(unit: TimePeriod) -> Self {
        match unit {
            TimePeriod::Day | TimePeriod::Week => Self::Day,
            TimePeriod::Month | TimePeriod::Year => Self::Month,
        }
    }
}

/// UTC start plus grain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct At {
    pub start: Instant,
    pub grain: Grain,
}

impl At {
    pub fn now() -> Self {
        Self {
            start: Instant::now(),
            grain: Grain::Instant,
        }
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

pub fn parse_at(input: &str, tz: &TimeZone) -> Result<At, Error> {
    match parse(input, tz)? {
        Parsed::Instant(ts) => Ok(At {
            start: Instant::from_timestamp(ts),
            grain: Grain::Instant,
        }),
        Parsed::Date(date) => Ok(At {
            start: Instant::from_timestamp(date_midnight(date, tz)?),
            grain: Grain::Day,
        }),
        Parsed::Month { year, month } => Ok(At {
            start: month_start(year, month, tz)?,
            grain: Grain::Month,
        }),
    }
}

pub fn display_local(at: Instant, tz: &TimeZone) -> Result<String, Error> {
    let zoned = at.timestamp().to_zoned(tz.clone());
    Ok(strtime::format("%Y-%m-%dT%H:%M:%S%:z", &zoned)?)
}

pub fn display_at(at: At, tz: &TimeZone) -> Result<String, Error> {
    match at.grain {
        Grain::Instant => display_local(at.start, tz),
        Grain::Day => Ok(local_civil(at.start, tz).to_string()),
        Grain::Month => {
            let date = local_civil(at.start, tz);
            Ok(format!("{:04}-{:02}", date.year(), date.month()))
        }
    }
}

/// Exclusive UTC end of this `at` value in `tz`. An instant occupies one
/// second so range overlap can use the same half-open test as day/month.
pub fn grain_end(at: At, tz: &TimeZone) -> Result<Instant, Error> {
    match at.grain {
        Grain::Instant => at.start.next_second(),
        Grain::Day => {
            let next = local_civil(at.start, tz).checked_add(jiff::Span::new().days(1))?;
            Ok(Instant::from_timestamp(date_midnight(next, tz)?))
        }
        Grain::Month => {
            let date = local_civil(at.start, tz);
            let first = Date::new(date.year(), date.month(), 1)?;
            let next = first.checked_add(jiff::Span::new().months(1))?;
            Ok(Instant::from_timestamp(date_midnight(next, tz)?))
        }
    }
}

fn from_bound(input: &str, tz: &TimeZone) -> Result<Instant, Error> {
    match parse(input, tz)? {
        Parsed::Instant(ts) => Ok(Instant::from_timestamp(ts)),
        Parsed::Date(date) => Ok(Instant::from_timestamp(date_midnight(date, tz)?)),
        Parsed::Month { year, month } => month_start(year, month, tz),
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

    pub fn exclusive_to(self) -> Result<Option<Instant>, Error> {
        match self.to {
            None => Ok(None),
            Some(ToBound::Exclusive(end)) => Ok(Some(end)),
            Some(ToBound::Inclusive(end)) => Ok(Some(end.next_second()?)),
        }
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
        Parsed::Month { year, month } => Ok(ToBound::Exclusive(month_end(year, month, tz)?)),
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
    Month { year: i16, month: i8 },
}

fn parse(input: &str, tz: &TimeZone) -> Result<Parsed, Error> {
    let invalid_time = || Error::Usage(Usage::InvalidTime(input.to_string()));
    let invalid_date = || Error::Usage(Usage::InvalidDate(input.to_string()));
    if looks_like_date(input) {
        let date: Date = input.parse().map_err(|_| invalid_date())?;
        return Ok(Parsed::Date(date));
    }
    if looks_like_month(input) {
        let year: i16 = input[..4].parse().map_err(|_| invalid_date())?;
        let month: i8 = input[5..].parse().map_err(|_| invalid_date())?;
        Date::new(year, month, 1).map_err(|_| invalid_date())?;
        return Ok(Parsed::Month { year, month });
    }
    let Some((date, rest)) = split_date_time(input) else {
        return Err(invalid_time());
    };
    if !looks_like_date(date) {
        return Err(invalid_time());
    }
    let Some((hms, offset)) = split_time_offset(rest) else {
        return Err(invalid_time());
    };
    let Some((hour, minute, second)) = parse_hms(hms) else {
        return Err(invalid_time());
    };
    let date: Date = date.parse().map_err(|_| invalid_time())?;
    let time = Time::new(hour, minute, second, 0).map_err(|_| invalid_time())?;
    let dt = date.to_datetime(time);
    let zone = match offset {
        None => tz.clone(),
        Some("Z") => TimeZone::UTC,
        Some(raw) => {
            let seconds = parse_offset_seconds(raw).ok_or_else(invalid_time)?;
            TimeZone::fixed(Offset::from_seconds(seconds).map_err(|_| invalid_time())?)
        }
    };
    let zoned = dt.to_zoned(zone).map_err(|_| invalid_time())?;
    Ok(Parsed::Instant(zoned.timestamp()))
}

fn split_date_time(input: &str) -> Option<(&str, &str)> {
    input.split_once('T').or_else(|| input.split_once(' '))
}

fn split_time_offset(rest: &str) -> Option<(&str, Option<&str>)> {
    if rest.ends_with('Z') {
        return Some((rest.strip_suffix('Z')?, Some("Z")));
    }
    match rest.rfind(['+', '-']) {
        None => Some((rest, None)),
        Some(0) => None,
        Some(i) => Some((&rest[..i], Some(&rest[i..]))),
    }
}

fn parse_hms(s: &str) -> Option<(i8, i8, i8)> {
    if s.len() == 5 && s.as_bytes()[2] == b':' && s.bytes().all(|b| b == b':' || b.is_ascii_digit())
    {
        let hour = s[..2].parse().ok()?;
        let minute = s[3..].parse().ok()?;
        return Some((hour, minute, 0));
    }
    if looks_like_hms(s) {
        let hour = s[..2].parse().ok()?;
        let minute = s[3..5].parse().ok()?;
        let second = s[6..].parse().ok()?;
        return Some((hour, minute, second));
    }
    None
}

fn parse_offset_seconds(s: &str) -> Option<i32> {
    let sign: i32 = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &s[1..];
    let (hour, minute): (i32, i32) = if rest.len() == 2 && rest.bytes().all(|b| b.is_ascii_digit())
    {
        (rest.parse().ok()?, 0)
    } else if rest.len() == 4 && rest.bytes().all(|b| b.is_ascii_digit()) {
        (rest[..2].parse().ok()?, rest[2..].parse().ok()?)
    } else if rest.len() == 5
        && rest.as_bytes()[2] == b':'
        && rest.bytes().all(|b| b == b':' || b.is_ascii_digit())
    {
        (rest[..2].parse().ok()?, rest[3..].parse().ok()?)
    } else {
        return None;
    };
    if !(0..=59).contains(&minute) {
        return None;
    }
    sign.checked_mul(
        hour.checked_mul(3600)?
            .checked_add(minute.checked_mul(60)?)?,
    )
}

fn looks_like_date(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes()[4] == b'-'
        && s.as_bytes()[7] == b'-'
        && s.bytes().all(|b| b == b'-' || b.is_ascii_digit())
}

fn looks_like_month(s: &str) -> bool {
    s.len() == 7 && s.as_bytes()[4] == b'-' && s.bytes().all(|b| b == b'-' || b.is_ascii_digit())
}

fn month_start(year: i16, month: i8, tz: &TimeZone) -> Result<Instant, Error> {
    let date = Date::new(year, month, 1)?;
    Ok(Instant::from_timestamp(date_midnight(date, tz)?))
}

fn month_end(year: i16, month: i8, tz: &TimeZone) -> Result<Instant, Error> {
    let date = Date::new(year, month, 1)?;
    let next = date.checked_add(jiff::Span::new().months(1))?;
    Ok(Instant::from_timestamp(date_midnight(next, tz)?))
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
            let err = parse_at(input, &tz).unwrap_err();
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
        let at = parse_at("2026-08-22T08:14:00+10:00", &tz).unwrap().start;
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
        let z = parse_at("2026-08-21T22:14:00Z", &tz).unwrap();
        let offset = parse_at("2026-08-22T08:14:00+10:00", &tz).unwrap();
        let naive = parse_at("2026-08-22T08:14:00", &tz).unwrap();
        assert_eq!(z, offset);
        assert_eq!(z, naive);
        assert_eq!(display_at(z, &tz).unwrap(), "2026-08-22T08:14:00+10:00");
        for input in [
            "2026-08-22T08:14:00+10:00",
            "2026-08-22T08:14:00+1000",
            "2026-08-22T08:14:00+10",
            "2026-08-22T08:14+10:00",
            "2026-08-22T08:14+1000",
            "2026-08-22T08:14+10",
            "2026-08-22 08:14:00+10:00",
            "2026-08-22 08:14+10",
            "2026-08-21T22:14:00Z",
            "2026-08-21T22:14Z",
            "2026-08-21 22:14:00Z",
            "2026-08-22T08:14:00",
            "2026-08-22T08:14",
            "2026-08-22 08:14:00",
            "2026-08-22 08:14",
        ] {
            assert_eq!(parse_at(input, &tz).unwrap(), z, "{input}");
        }
    }

    #[test]
    fn negative_offset_is_an_instant() {
        let tz = melbourne();
        let utc = parse_at("2026-08-22T13:14:00Z", &tz).unwrap();
        for input in [
            "2026-08-22T08:14:00-05:00",
            "2026-08-22T08:14:00-0500",
            "2026-08-22T08:14:00-05",
            "2026-08-22T08:14-05",
        ] {
            assert_eq!(parse_at(input, &tz).unwrap(), utc, "{input}");
        }
    }

    #[test]
    fn date_and_month_are_grains() {
        let tz = melbourne();
        let day = parse_at("2026-08-22", &tz).unwrap();
        assert_eq!(day.grain, Grain::Day);
        assert_eq!(display_at(day, &tz).unwrap(), "2026-08-22");
        assert_eq!(
            display_local(day.start, &tz).unwrap(),
            "2026-08-22T00:00:00+10:00"
        );
        let month = parse_at("2026-08", &tz).unwrap();
        assert_eq!(month.grain, Grain::Month);
        assert_eq!(display_at(month, &tz).unwrap(), "2026-08");
        assert_eq!(
            display_local(month.start, &tz).unwrap(),
            "2026-08-01T00:00:00+10:00"
        );
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
        let month_range = Range::parse(Some("2026-08"), Some("2026-08"), &tz).unwrap();
        assert_eq!(
            display_local(month_range.from.unwrap(), &tz).unwrap(),
            "2026-08-01T00:00:00+10:00"
        );
        let Some(ToBound::Exclusive(month_to)) = month_range.to else {
            panic!("{:?}", month_range.to);
        };
        assert_eq!(
            display_local(month_to, &tz).unwrap(),
            "2026-09-01T00:00:00+10:00"
        );
    }

    #[test]
    fn grain_end_is_exclusive() {
        let tz = melbourne();
        let instant = parse_at("2026-08-22T08:14:00+10:00", &tz).unwrap();
        assert_eq!(
            display_local(grain_end(instant, &tz).unwrap(), &tz).unwrap(),
            "2026-08-22T08:14:01+10:00"
        );
        let day = parse_at("2026-08-22", &tz).unwrap();
        assert_eq!(
            display_local(grain_end(day, &tz).unwrap(), &tz).unwrap(),
            "2026-08-23T00:00:00+10:00"
        );
        let month = parse_at("2026-08", &tz).unwrap();
        assert_eq!(
            display_local(grain_end(month, &tz).unwrap(), &tz).unwrap(),
            "2026-09-01T00:00:00+10:00"
        );
        let dst = parse_at("2026-10-04", &tz).unwrap();
        let hours = (grain_end(dst, &tz).unwrap().timestamp().as_second()
            - dst.start.timestamp().as_second())
            / 3600;
        assert_eq!(hours, 23);
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
        let gap = parse_at("2026-10-04T02:30:00", &tz).unwrap();
        let later = parse_at("2026-10-04T03:30:00", &tz).unwrap();
        assert_eq!(gap, later);
        assert_eq!(display_at(gap, &tz).unwrap(), "2026-10-04T03:30:00+11:00");
    }

    #[test]
    fn rejects_bad_time_and_date_shapes() {
        let tz = melbourne();
        for input in [
            "2026-08-22t08:14:00",
            "2026-08-22T08:14:00z",
            "2026-08-22T25:00:00",
            "2026-08-22T08:61:00",
            "2026-08-22T08:14:00.5Z",
            "2026-08-22T08",
            "2026-08-22T08:14:00+10:0",
            "2026-08-22T08:14:00+1",
            "2026-08-22T08:14:00 +10:00",
        ] {
            let err = parse_at(input, &tz).unwrap_err();
            assert!(
                matches!(err, Error::Usage(Usage::InvalidTime(ref s)) if s == input),
                "{input}: {err}"
            );
        }
        for input in ["2026-02-30", "2025-02-29", "2026-13"] {
            let err = parse_at(input, &tz).unwrap_err();
            assert!(
                matches!(err, Error::Usage(Usage::InvalidDate(ref s)) if s == input),
                "{input}: {err}"
            );
        }
        for input in ["2026", "2026-W34", "2026-Q3"] {
            let err = parse_at(input, &tz).unwrap_err();
            assert!(
                matches!(err, Error::Usage(Usage::InvalidTime(ref s)) if s == input),
                "{input}: {err}"
            );
        }
    }
}
