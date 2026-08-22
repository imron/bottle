use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::fmt::strtime;
use jiff::tz::TimeZone;

use crate::error::Error;

pub fn now_stored() -> Result<String, Error> {
    format_stored(Timestamp::now())
}

pub fn parse_instant(input: &str) -> Result<String, Error> {
    match parse(input)? {
        Parsed::Instant(ts) => format_stored(ts),
        Parsed::Date(_) => Err(Error::usage("date-only is a query bound, not an instant")),
    }
}

pub fn display_local(stored: &str) -> Result<String, Error> {
    let ts = parse_stored(stored)?;
    let zoned = ts.to_zoned(system_tz());
    strtime::format("%Y-%m-%dT%H:%M:%S%:z", &zoned).map_err(|e| Error::fail(e.to_string()))
}

pub fn from_bound(input: &str) -> Result<String, Error> {
    match parse(input)? {
        Parsed::Instant(ts) => format_stored(ts),
        Parsed::Date(date) => format_stored(date_midnight(date)?),
    }
}

#[derive(Debug, Clone)]
pub enum ToBound {
    Inclusive(String),
    Exclusive(String),
}

#[derive(Debug, Clone, Default)]
pub struct Range {
    pub from: Option<String>,
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
        let start = format_stored(date_midnight(today)?)?;
        let next = today
            .checked_add(jiff::Span::new().days(1))
            .map_err(|e| Error::fail(e.to_string()))?;
        let end = format_stored(date_midnight(next)?)?;
        Ok(Self {
            from: Some(start),
            to: Some(ToBound::Exclusive(end)),
        })
    }
}

fn to_bound(input: &str) -> Result<ToBound, Error> {
    match parse(input)? {
        Parsed::Instant(ts) => Ok(ToBound::Inclusive(format_stored(ts)?)),
        Parsed::Date(date) => {
            let next = date
                .checked_add(jiff::Span::new().days(1))
                .map_err(|e| Error::fail(e.to_string()))?;
            Ok(ToBound::Exclusive(format_stored(date_midnight(next)?)?))
        }
    }
}

pub fn local_civil(stored: &str) -> Result<Date, Error> {
    let ts = parse_stored(stored)?;
    Ok(ts.to_zoned(system_tz()).date())
}

enum Parsed {
    Instant(Timestamp),
    Date(Date),
}

fn parse(input: &str) -> Result<Parsed, Error> {
    if input.contains(' ') {
        return Err(Error::usage("time must use T, not a space"));
    }
    if looks_like_date(input) {
        let date: Date = input
            .parse()
            .map_err(|_| Error::usage(format!("invalid date: {input}")))?;
        return Ok(Parsed::Date(date));
    }
    let Some((date, rest)) = input.split_once('T') else {
        return Err(Error::usage(format!("invalid time: {input}")));
    };
    if !looks_like_date(date) || !looks_like_hms(&rest[..rest.len().min(8)]) {
        return Err(Error::usage(format!("invalid time: {input}")));
    }
    if rest.len() == 8 {
        let dt: DateTime = strtime::parse("%Y-%m-%dT%H:%M:%S", input)
            .and_then(|p| p.to_datetime())
            .map_err(|_| Error::usage(format!("invalid time: {input}")))?;
        let zoned = dt
            .to_zoned(system_tz())
            .map_err(|e| Error::usage(e.to_string()))?;
        return Ok(Parsed::Instant(zoned.timestamp()));
    }
    if rest.ends_with('Z') {
        if rest.len() != 9 {
            return Err(Error::usage(format!("invalid time: {input}")));
        }
        let ts: Timestamp = input
            .parse()
            .map_err(|_| Error::usage(format!("invalid time: {input}")))?;
        return Ok(Parsed::Instant(ts));
    }
    let Some(sign_at) = rest.rfind(['+', '-']) else {
        return Err(Error::usage(format!("invalid time: {input}")));
    };
    if sign_at != 8 {
        return Err(Error::usage(format!("invalid time: {input}")));
    }
    let offset = &rest[sign_at..];
    if offset.len() != 6 || offset.as_bytes().get(3) != Some(&b':') {
        return Err(Error::usage("offset must include a colon (+10:00)"));
    }
    let zoned = strtime::parse("%Y-%m-%dT%H:%M:%S%:z", input)
        .and_then(|p| p.to_zoned())
        .map_err(|_| Error::usage(format!("invalid time: {input}")))?;
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

fn parse_stored(stored: &str) -> Result<Timestamp, Error> {
    stored
        .parse()
        .map_err(|_| Error::fail(format!("corrupt stored time: {stored}")))
}

fn format_stored(ts: Timestamp) -> Result<String, Error> {
    let zoned = ts.to_zoned(TimeZone::UTC);
    strtime::format("%Y-%m-%dT%H:%M:%SZ", &zoned).map_err(|e| Error::fail(e.to_string()))
}

fn date_midnight(date: Date) -> Result<Timestamp, Error> {
    date.to_zoned(system_tz())
        .map(|z| z.timestamp())
        .map_err(|e| Error::fail(e.to_string()))
}

fn system_tz() -> TimeZone {
    TimeZone::system()
}
