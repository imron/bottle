use rusqlite::params;

use crate::error::Error;
use crate::spec::{
    Field, FieldType, Spec, fold_enum, is_ident, is_reserved, is_time_group, parse_number,
};
use crate::time;
use crate::tsv;

#[derive(Clone)]
pub(crate) enum SqlVal {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
}

impl SqlVal {
    pub(crate) fn as_param(&self) -> rusqlite::types::ToSqlOutput<'_> {
        use rusqlite::types::{ToSqlOutput, Value, ValueRef};
        match self {
            SqlVal::Null => ToSqlOutput::Borrowed(ValueRef::Null),
            SqlVal::Int(n) => ToSqlOutput::Owned(Value::Integer(*n)),
            SqlVal::Real(n) => ToSqlOutput::Owned(Value::Real(*n)),
            SqlVal::Text(s) => ToSqlOutput::Borrowed(ValueRef::Text(s.as_bytes())),
        }
    }
}

impl rusqlite::ToSql for SqlVal {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.as_param())
    }
}

pub(crate) fn sql_type(t: FieldType) -> &'static str {
    match t {
        FieldType::Number => "REAL",
        FieldType::Text | FieldType::Enum => "TEXT",
    }
}

pub(crate) fn field_sql(field: &Field, value: &str) -> Result<SqlVal, Error> {
    match field.type_ {
        FieldType::Text => {
            if value.contains('\t') || value.contains('\n') {
                return Err(Error::fail(format!(
                    "text {} may not contain tab or newline",
                    field.name
                )));
            }
            Ok(SqlVal::Text(value.to_string()))
        }
        FieldType::Number => Ok(SqlVal::Real(parse_number(value)?)),
        FieldType::Enum => {
            let folded = fold_enum(value);
            let Some(values) = &field.values else {
                return Err(Error::fail(format!("enum {} has no values", field.name)));
            };
            if !values.iter().any(|v| v == &folded) {
                return Err(Error::fail(format!(
                    "invalid {} value: {value}",
                    field.name
                )));
            }
            Ok(SqlVal::Text(folded))
        }
    }
}

pub(crate) fn validate_link_name(name: &str, spec: &Spec) -> Result<(), Error> {
    if !is_ident(name) {
        return Err(Error::fail(format!("invalid link name: {name}")));
    }
    if is_reserved(name) || is_time_group(name) {
        return Err(Error::fail(format!("reserved link name: {name}")));
    }
    if spec.field(name).is_some() {
        return Err(Error::fail(format!(
            "link name collides with field: {name}"
        )));
    }
    Ok(())
}

pub(crate) fn insert_links(
    tx: &rusqlite::Transaction<'_>,
    schema: &str,
    id: i64,
    links: &[(String, String, i64)],
) -> Result<(), Error> {
    for (name, to_schema, to_id) in links {
        tx.execute(
            "INSERT INTO links (from_schema, from_id, name, to_schema, to_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![schema, id, name, to_schema, to_id],
        )?;
    }
    Ok(())
}

pub(crate) fn format_links(links: &[(String, String, i64)]) -> String {
    let mut pairs: Vec<(String, String)> = links
        .iter()
        .map(|(n, s, i)| (n.clone(), format!("{s}/{i}")))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    format_links_pairs(&pairs)
}

pub(crate) fn format_links_pairs(links: &[(String, String)]) -> String {
    links
        .iter()
        .map(|(n, t)| format!("{n}={t}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn write_id_row(id: i64, stored_at: &str, links: &str) -> Result<String, Error> {
    let at = time::display_local(stored_at)?;
    Ok(tsv::table(
        &["id", "at", "links"],
        &[vec![id.to_string(), at, links.to_string()]],
    ))
}
