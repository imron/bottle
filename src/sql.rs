use crate::error::{Error, Fail};
use crate::ledger::{Agent, FieldValue, NonEmptyFieldValue};
use crate::spec::{EntryId, EnumValue, LinkName, SchemaName, parse_number};
use crate::time::Instant;
use jiff::fmt::strtime;
use jiff::tz::TimeZone;
use rusqlite::types::{ToSqlOutput, Value, ValueRef};
use rust_decimal::Decimal;

#[derive(Clone)]
pub enum SqlVal {
    Null,
    Int(i64),
    Text(String),
}

impl SqlVal {
    pub fn as_param(&self) -> ToSqlOutput<'_> {
        match self {
            SqlVal::Null => ToSqlOutput::Borrowed(ValueRef::Null),
            SqlVal::Int(n) => ToSqlOutput::Owned(Value::Integer(*n)),
            SqlVal::Text(s) => ToSqlOutput::Borrowed(ValueRef::Text(s.as_bytes())),
        }
    }

    pub fn from_field(value: &FieldValue) -> Self {
        match value {
            FieldValue::Empty => SqlVal::Null,
            FieldValue::Text(s) => SqlVal::Text(s.clone()),
            FieldValue::Number(n) => SqlVal::Text(n.to_string()),
            FieldValue::Enum(v) => SqlVal::Text(v.to_string()),
        }
    }

    pub fn from_filter(value: &NonEmptyFieldValue) -> Self {
        match value {
            NonEmptyFieldValue::Text(s) => SqlVal::Text(s.clone()),
            NonEmptyFieldValue::Number(n) => SqlVal::Text(n.to_string()),
            NonEmptyFieldValue::Enum(v) => SqlVal::Text(v.to_string()),
        }
    }
}

pub fn table_name(schema: &SchemaName) -> String {
    format!("entry_{}", schema.as_str().replace('.', "_"))
}

pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn instant_to_sql(at: Instant) -> Result<String, Error> {
    let zoned = at.timestamp().to_zoned(TimeZone::UTC);
    Ok(strtime::format("%Y-%m-%dT%H:%M:%SZ", &zoned)?)
}

pub struct StoredTime(pub String);
pub struct StoredSchemaName(pub String);
pub struct StoredLinkName(pub String);
pub struct StoredLinkSchema(pub String);
pub struct StoredNumber(pub String);
pub struct StoredEnum(pub String);
pub struct StoredAgent(pub String);
pub struct StoredText(pub String);
pub struct StoredEntryId(pub i64);

macro_rules! try_from_stored_name {
    ($stored:ident, $ty:ty, $fail:ident) => {
        impl TryFrom<$stored> for $ty {
            type Error = Error;

            fn try_from($stored(raw): $stored) -> Result<Self, Error> {
                match <$ty>::parse(&raw) {
                    Ok(value) if value.as_str() == raw => Ok(value),
                    _ => Err(Error::Fail(Fail::$fail(raw))),
                }
            }
        }
    };
}

impl TryFrom<StoredTime> for Instant {
    type Error = Error;

    fn try_from(StoredTime(raw): StoredTime) -> Result<Self, Error> {
        let Ok(ts) = raw.parse() else {
            return Err(Error::Fail(Fail::CorruptStoredTime(raw)));
        };
        let Ok(at) = Instant::from_timestamp(ts) else {
            return Err(Error::Fail(Fail::CorruptStoredTime(raw)));
        };
        match instant_to_sql(at) {
            Ok(canonical) if canonical == raw => Ok(at),
            _ => Err(Error::Fail(Fail::CorruptStoredTime(raw))),
        }
    }
}

try_from_stored_name!(StoredSchemaName, SchemaName, CorruptSchemaName);
try_from_stored_name!(StoredLinkName, LinkName, CorruptLinkName);
try_from_stored_name!(StoredLinkSchema, SchemaName, CorruptLinkSchema);
try_from_stored_name!(StoredEnum, EnumValue, CorruptStoredEnum);
try_from_stored_name!(StoredAgent, Agent, CorruptStoredAgent);

impl TryFrom<StoredText> for String {
    type Error = Error;

    fn try_from(StoredText(raw): StoredText) -> Result<Self, Error> {
        if raw.contains('\t') || raw.contains('\n') {
            Err(Error::Fail(Fail::CorruptStoredText(raw)))
        } else {
            Ok(raw)
        }
    }
}

impl TryFrom<StoredEntryId> for EntryId {
    type Error = Error;

    fn try_from(StoredEntryId(raw): StoredEntryId) -> Result<Self, Error> {
        Self::from_raw(raw).ok_or(Error::Fail(Fail::CorruptStoredId(raw)))
    }
}

impl TryFrom<StoredNumber> for Decimal {
    type Error = Error;

    fn try_from(StoredNumber(raw): StoredNumber) -> Result<Self, Error> {
        parse_number(&raw).map_err(|_| Error::Fail(Fail::CorruptStoredNumber(raw)))
    }
}

pub fn sql_default(value: &NonEmptyFieldValue) -> String {
    let raw = match value {
        NonEmptyFieldValue::Text(s) => s.clone(),
        NonEmptyFieldValue::Number(n) => n.to_string(),
        NonEmptyFieldValue::Enum(v) => v.to_string(),
    };
    format!("'{}'", raw.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_values_are_corrupt_when_invalid() {
        assert_eq!(
            Decimal::try_from(StoredNumber("nope".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored number: nope"
        );
        assert_eq!(
            Instant::try_from(StoredTime("nope".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored time: nope"
        );
        assert_eq!(
            SchemaName::try_from(StoredSchemaName("Nope".into()))
                .unwrap_err()
                .to_string(),
            "corrupt schema name: Nope"
        );
        assert_eq!(
            LinkName::try_from(StoredLinkName("Nope".into()))
                .unwrap_err()
                .to_string(),
            "corrupt link name: Nope"
        );
        assert_eq!(
            SchemaName::try_from(StoredLinkSchema("Nope".into()))
                .unwrap_err()
                .to_string(),
            "corrupt link schema: Nope"
        );
        assert_eq!(
            EnumValue::try_from(StoredEnum("a,b".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored enum: a,b"
        );
        assert_eq!(
            Agent::try_from(StoredAgent("a\tb".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored agent: a\tb"
        );
        assert_eq!(
            Agent::try_from(StoredAgent("".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored agent: "
        );
        assert_eq!(
            String::try_from(StoredText("a\tb".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored text: a\tb"
        );
        assert_eq!(
            String::try_from(StoredText("a\nb".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored text: a\nb"
        );
        assert_eq!(
            EntryId::try_from(StoredEntryId(0)).unwrap_err().to_string(),
            "corrupt stored id: 0"
        );
    }

    #[test]
    fn stored_values_are_corrupt_when_not_canonical() {
        assert_eq!(
            SchemaName::try_from(StoredSchemaName("foo_bar".into()))
                .unwrap_err()
                .to_string(),
            "corrupt schema name: foo_bar"
        );
        assert_eq!(
            SchemaName::try_from(StoredLinkSchema("foo_bar".into()))
                .unwrap_err()
                .to_string(),
            "corrupt link schema: foo_bar"
        );
        assert_eq!(
            EnumValue::try_from(StoredEnum("Breakfast".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored enum: Breakfast"
        );
        assert_eq!(
            EnumValue::try_from(StoredEnum(" lunch".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored enum:  lunch"
        );
        assert_eq!(
            Agent::try_from(StoredAgent(" coach ".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored agent:  coach "
        );
        assert_eq!(
            Decimal::try_from(StoredNumber("1e3".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored number: 1e3"
        );
        assert_eq!(
            Decimal::try_from(StoredNumber("01".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored number: 01"
        );
        assert_eq!(
            Instant::try_from(StoredTime("2026-08-22T08:14:00+00:00".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored time: 2026-08-22T08:14:00+00:00"
        );
        assert_eq!(
            Instant::try_from(StoredTime("2026-08-22T08:14:00z".into()))
                .unwrap_err()
                .to_string(),
            "corrupt stored time: 2026-08-22T08:14:00z"
        );
    }

    #[test]
    fn stored_values_accept_canonical_form() {
        assert_eq!(
            SchemaName::try_from(StoredSchemaName("foo.bar".into()))
                .unwrap()
                .as_str(),
            "foo.bar"
        );
        assert_eq!(
            LinkName::try_from(StoredLinkName("session".into()))
                .unwrap()
                .as_str(),
            "session"
        );
        assert_eq!(
            EnumValue::try_from(StoredEnum("breakfast".into()))
                .unwrap()
                .as_str(),
            "breakfast"
        );
        assert_eq!(
            Agent::try_from(StoredAgent("coach".into()))
                .unwrap()
                .as_str(),
            "coach"
        );
        assert_eq!(
            String::try_from(StoredText("4 eggs".into())).unwrap(),
            "4 eggs"
        );
        assert_eq!(
            Decimal::try_from(StoredNumber("39.60".into()))
                .unwrap()
                .to_string(),
            "39.60"
        );
        let at = Instant::try_from(StoredTime("2026-08-22T08:14:00Z".into())).unwrap();
        assert_eq!(instant_to_sql(at).unwrap(), "2026-08-22T08:14:00Z");
    }
}
