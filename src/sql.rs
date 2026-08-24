use crate::error::Error;
use crate::ledger::FieldValue;
use crate::spec::{FieldType, SchemaName};
use crate::time::Instant;
use jiff::fmt::strtime;
use jiff::tz::TimeZone;
use rusqlite::types::{ToSqlOutput, Value, ValueRef};

#[derive(Clone)]
pub(crate) enum SqlVal {
    Null,
    Int(i64),
    Text(String),
}

impl SqlVal {
    pub(crate) fn as_param(&self) -> ToSqlOutput<'_> {
        match self {
            SqlVal::Null => ToSqlOutput::Borrowed(ValueRef::Null),
            SqlVal::Int(n) => ToSqlOutput::Owned(Value::Integer(*n)),
            SqlVal::Text(s) => ToSqlOutput::Borrowed(ValueRef::Text(s.as_bytes())),
        }
    }

    pub(crate) fn from_field(value: &FieldValue) -> Self {
        match value {
            FieldValue::Empty => SqlVal::Null,
            FieldValue::Text(s) => SqlVal::Text(s.clone()),
            FieldValue::Number(n) => SqlVal::Text(n.to_string()),
        }
    }
}

impl rusqlite::ToSql for SqlVal {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.as_param())
    }
}

pub(crate) fn sql_type(t: FieldType) -> &'static str {
    match t {
        FieldType::Number | FieldType::Text | FieldType::Enum => "TEXT",
    }
}

pub(crate) fn table_name(schema: &SchemaName) -> String {
    schema.as_str().replace('.', "_")
}

pub(crate) fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub(crate) fn instant_to_sql(at: Instant) -> Result<String, Error> {
    let zoned = at.timestamp().to_zoned(TimeZone::UTC);
    strtime::format("%Y-%m-%dT%H:%M:%SZ", &zoned).map_err(|e| Error::fail(e.to_string()))
}

pub(crate) fn instant_from_sql(raw: String) -> Result<Instant, Error> {
    let ts = raw
        .parse()
        .map_err(|_| Error::fail(format!("corrupt stored time: {raw}")))?;
    Ok(Instant::from_timestamp(ts))
}

pub(crate) fn sql_default(type_: FieldType, def: &str) -> String {
    match type_ {
        FieldType::Number | FieldType::Text | FieldType::Enum => {
            format!("'{}'", def.replace('\'', "''"))
        }
    }
}
