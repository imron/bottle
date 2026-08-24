use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::fmt::strtime;
use jiff::tz::TimeZone;

use crate::error::{Error, Fail, Usage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(Timestamp);

impl Instant {
    pub fn now() -> Self {
        Self(Timestamp::now())
    }

    pub fn from_timestamp(ts: Timestamp) -> Self {
        Self(ts)
    }

    pub fn timestamp(self) -> Timestamp {
        self.0
    }
}

pub fn parse_instant(input: &str) -> Result<Instant, Error> {
    match parse(input)? {
        Parsed::Instant(ts) => Ok(Instant(ts)),
        Parsed::Date(_) => Err(Error::Usage(Usage::DateOnlyNotInstant)),
    }
}

pub fn display_local(at: Instant) -> Result<String, Error> {
    let zoned = at.0.to_zoned(system_tz());
    strtime::format("%Y-%m-%dT%H:%M:%S%:z", &zoned)
        .map_err(|e| Error::Fail(Fail::Time(e.to_string())))
}

fn from_bound(input: &str) -> Result<Instant, Error> {
    match parse(input)? {
        Parsed::Instant(ts) => Ok(Instant(ts)),
        Parsed::Date(date) => Ok(Instant(date_midnight(date)?)),
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
    pub fn parse(from: Option<&str>, to: Option<&str>) -> Result<Self, Error> {
        Ok(Self {
            from: from.map(from_bound).transpose()?,
            to: to.map(to_bound).transpose()?,
        })
    }

    pub fn today() -> Result<Self, Error> {
        let today = Timestamp::now().to_zoned(system_tz()).date();
        let start = Instant(date_midnight(today)?);
        let next = today
            .checked_add(jiff::Span::new().days(1))
            .map_err(|e| Error::Fail(Fail::Time(e.to_string())))?;
        let end = Instant(date_midnight(next)?);
        Ok(Self {
            from: Some(start),
            to: Some(ToBound::Exclusive(end)),
        })
    }
}

fn to_bound(input: &str) -> Result<ToBound, Error> {
    match parse(input)? {
        Parsed::Instant(ts) => Ok(ToBound::Inclusive(Instant(ts))),
        Parsed::Date(date) => {
            let next = date
                .checked_add(jiff::Span::new().days(1))
                .map_err(|e| Error::Fail(Fail::Time(e.to_string())))?;
            Ok(ToBound::Exclusive(Instant(date_midnight(next)?)))
        }
    }
}

pub fn local_civil(at: Instant) -> Date {
    at.0.to_zoned(system_tz()).date()
}

enum Parsed {
    Instant(Timestamp),
    Date(Date),
}

fn parse(input: &str) -> Result<Parsed, Error> {
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
    if !looks_like_date(date) || !looks_like_hms(&rest[..rest.len().min(8)]) {
        return Err(Error::Usage(Usage::InvalidTime(input.to_string())));
    }
    if rest.len() == 8 {
        let dt: DateTime = strtime::parse("%Y-%m-%dT%H:%M:%S", input)
            .and_then(|p| p.to_datetime())
            .map_err(|_| Error::Usage(Usage::InvalidTime(input.to_string())))?;
        let zoned = dt
            .to_zoned(system_tz())
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

fn date_midnight(date: Date) -> Result<Timestamp, Error> {
    date.to_zoned(system_tz())
        .map(|z| z.timestamp())
        .map_err(|e| Error::Fail(Fail::Time(e.to_string())))
}

fn system_tz() -> TimeZone {
    TimeZone::system()
}
