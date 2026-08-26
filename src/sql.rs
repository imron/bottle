use crate::error::{Error, Fail};
use crate::ledger::{Agent, FieldValue};
use crate::spec::{EnumValue, FieldKind, LinkName, SchemaName};
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
}

pub fn sql_type(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Number | FieldKind::Text | FieldKind::Enum(_) => "TEXT",
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

impl TryFrom<StoredTime> for Instant {
    type Error = Error;

    fn try_from(StoredTime(raw): StoredTime) -> Result<Self, Error> {
        match raw.parse() {
            Ok(ts) => Ok(Instant::from_timestamp(ts)),
            Err(_) => Err(Error::Fail(Fail::CorruptStoredTime(raw))),
        }
    }
}

impl TryFrom<StoredSchemaName> for SchemaName {
    type Error = Error;

    fn try_from(StoredSchemaName(name): StoredSchemaName) -> Result<Self, Error> {
        match SchemaName::parse(&name) {
            Ok(name) => Ok(name),
            Err(_) => Err(Error::Fail(Fail::CorruptSchemaName(name))),
        }
    }
}

impl TryFrom<StoredLinkName> for LinkName {
    type Error = Error;

    fn try_from(StoredLinkName(name): StoredLinkName) -> Result<Self, Error> {
        match LinkName::parse(&name) {
            Ok(name) => Ok(name),
            Err(_) => Err(Error::Fail(Fail::CorruptLinkName(name))),
        }
    }
}

impl TryFrom<StoredLinkSchema> for SchemaName {
    type Error = Error;

    fn try_from(StoredLinkSchema(name): StoredLinkSchema) -> Result<Self, Error> {
        match SchemaName::parse(&name) {
            Ok(name) => Ok(name),
            Err(_) => Err(Error::Fail(Fail::CorruptLinkSchema(name))),
        }
    }
}

impl TryFrom<StoredNumber> for Decimal {
    type Error = Error;

    fn try_from(StoredNumber(raw): StoredNumber) -> Result<Self, Error> {
        match raw.parse() {
            Ok(n) => Ok(n),
            Err(_) => Err(Error::Fail(Fail::CorruptStoredNumber(raw))),
        }
    }
}

impl TryFrom<StoredEnum> for EnumValue {
    type Error = Error;

    fn try_from(StoredEnum(raw): StoredEnum) -> Result<Self, Error> {
        match EnumValue::parse(&raw) {
            Ok(value) => Ok(value),
            Err(_) => Err(Error::Fail(Fail::CorruptStoredEnum(raw))),
        }
    }
}

impl TryFrom<StoredAgent> for Agent {
    type Error = Error;

    fn try_from(StoredAgent(raw): StoredAgent) -> Result<Self, Error> {
        match Agent::parse(&raw) {
            Ok(agent) => Ok(agent),
            Err(_) => Err(Error::Fail(Fail::CorruptStoredAgent(raw))),
        }
    }
}

pub fn sql_default(value: &FieldValue) -> String {
    let raw = match value {
        FieldValue::Empty => String::new(),
        FieldValue::Text(s) => s.clone(),
        FieldValue::Number(n) => n.to_string(),
        FieldValue::Enum(v) => v.to_string(),
    };
    format!("'{}'", raw.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_values_are_corrupt_when_invalid() {
        assert!(matches!(
            Decimal::try_from(StoredNumber("nope".into())),
            Err(Error::Fail(Fail::CorruptStoredNumber(_)))
        ));
        assert!(matches!(
            Instant::try_from(StoredTime("nope".into())),
            Err(Error::Fail(Fail::CorruptStoredTime(_)))
        ));
        assert!(matches!(
            SchemaName::try_from(StoredSchemaName("Nope".into())),
            Err(Error::Fail(Fail::CorruptSchemaName(_)))
        ));
        assert!(matches!(
            LinkName::try_from(StoredLinkName("Nope".into())),
            Err(Error::Fail(Fail::CorruptLinkName(_)))
        ));
        assert!(matches!(
            SchemaName::try_from(StoredLinkSchema("Nope".into())),
            Err(Error::Fail(Fail::CorruptLinkSchema(_)))
        ));
        assert!(matches!(
            EnumValue::try_from(StoredEnum("a,b".into())),
            Err(Error::Fail(Fail::CorruptStoredEnum(_)))
        ));
        assert!(matches!(
            Agent::try_from(StoredAgent("a\tb".into())),
            Err(Error::Fail(Fail::CorruptStoredAgent(_)))
        ));
    }
}
