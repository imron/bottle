use crate::error::{Error, Fail};
use crate::ledger::FieldValue;
use crate::spec::{FieldType, SchemaName};
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
    schema.as_str().replace('.', "_")
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

pub fn sql_default(type_: FieldType, def: &str) -> String {
    match type_ {
        FieldType::Number | FieldType::Text | FieldType::Enum => {
            format!("'{}'", def.replace('\'', "''"))
        }
    }
}
