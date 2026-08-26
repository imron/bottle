use crate::error::{Error, Fail};
use crate::ledger::FieldValue;
use crate::spec::{FieldType, LinkName, SchemaName};
use crate::time::Instant;
use jiff::fmt::strtime;
use jiff::tz::TimeZone;
use rusqlite::types::{ToSqlOutput, Value, ValueRef};

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

pub fn sql_type(t: FieldType) -> &'static str {
    match t {
        FieldType::Number | FieldType::Text | FieldType::Enum => "TEXT",
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

pub fn instant_from_sql(raw: String) -> Result<Instant, Error> {
    let ts = raw
        .parse()
        .map_err(|_| Error::Fail(Fail::CorruptStoredTime(raw.clone())))?;
    Ok(Instant::from_timestamp(ts))
}

pub fn schema_from_sql(name: String) -> Result<SchemaName, Error> {
    SchemaName::parse(&name).map_err(|_| Error::Fail(Fail::CorruptSchemaName(name)))
}

pub fn link_name_from_sql(name: String) -> Result<LinkName, Error> {
    LinkName::parse(&name).map_err(|_| Error::Fail(Fail::CorruptLinkName(name)))
}

pub fn link_schema_from_sql(name: String) -> Result<SchemaName, Error> {
    SchemaName::parse(&name).map_err(|_| Error::Fail(Fail::CorruptLinkSchema(name)))
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
