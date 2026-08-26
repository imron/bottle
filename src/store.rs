use std::collections::HashMap;

use rusqlite::OptionalExtension;

use crate::db::Connection;
use crate::error::{Error, Fail};
use crate::ledger::{Agent, Entry, FieldValue, Filter, Order, Schema, SchemaInfo};
use crate::spec::{EntryRef, FieldName, FieldType, Link, LinkName, SchemaName, Spec};
use crate::sql::{SqlVal, instant_from_sql, instant_to_sql, quote_ident, table_name};
use crate::time::{Range, ToBound};

pub struct Find<'a> {
    pub schema: &'a SchemaName,
    pub spec: &'a Spec,
    pub range: Range,
    pub agent: Option<&'a str>,
    pub include_ignored: bool,
    pub filters: &'a [Filter],
    pub order: Order,
    pub limit: Option<usize>,
}

pub fn list_schemas(conn: &impl Connection) -> Result<Vec<SchemaInfo>, Error> {
    let mut stmt = conn
        .as_ref()
        .prepare("SELECT name, retired FROM schemas ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let retired: i64 = row.get(1)?;
        Ok((name, retired))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (name, retired) = row?;
        let name = SchemaName::parse(&name)
            .map_err(|_| Error::Fail(Fail::CorruptSchemaName(name.clone())))?;
        out.push(SchemaInfo {
            name,
            retired: retired != 0,
        });
    }
    Ok(out)
}

pub fn load_schema(conn: &impl Connection, name: &SchemaName) -> Result<Schema, Error> {
    let row = conn
        .as_ref()
        .query_row(
            "SELECT spec, retired FROM schemas WHERE name = ?1",
            [name.as_str()],
            |row| {
                let spec: String = row.get(0)?;
                let retired: i64 = row.get(1)?;
                Ok((spec, retired))
            },
        )
        .optional()?;
    let Some((spec, retired)) = row else {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    };
    Ok(Schema {
        spec: Spec::parse_yaml(&spec)?,
        retired: retired != 0,
    })
}

pub fn inbound_link_count(conn: &impl Connection, name: &SchemaName) -> Result<i64, Error> {
    let n: i64 = conn.as_ref().query_row(
        "SELECT COUNT(*) FROM links WHERE to_schema = ?1",
        [name.as_str()],
        |row| row.get(0),
    )?;
    Ok(n)
}

pub fn get_entry(
    conn: &impl Connection,
    schema: &SchemaName,
    spec: &Spec,
    id: i64,
) -> Result<Option<Entry>, Error> {
    let sql = format!(
        "SELECT * FROM {} WHERE id = ?1",
        quote_ident(&table_name(schema))
    );
    let mut entry = {
        let mut stmt = conn.as_ref().prepare(&sql)?;
        let col_count = stmt.column_count();
        let names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
            .collect();
        let mut raw = stmt.query([id])?;
        match raw.next()? {
            Some(r) => read_entry(spec, &names, r)?,
            None => return Ok(None),
        }
    };
    attach_links(conn, schema, std::slice::from_mut(&mut entry))?;
    Ok(Some(entry))
}

pub fn find(conn: &impl Connection, q: Find<'_>) -> Result<Vec<Entry>, Error> {
    let mut sql = format!(
        "SELECT * FROM {} WHERE 1=1",
        quote_ident(&table_name(q.schema))
    );
    let mut bind: Vec<SqlVal> = Vec::new();
    if !q.include_ignored {
        sql.push_str(" AND ignored = 0");
    }
    if let Some(from) = q.range.from {
        bind.push(SqlVal::Text(instant_to_sql(from)?));
        sql.push_str(&format!(" AND at >= ?{}", bind.len()));
    }
    if let Some(to) = q.range.to {
        match to {
            ToBound::Inclusive(end) => {
                bind.push(SqlVal::Text(instant_to_sql(end)?));
                sql.push_str(&format!(" AND at <= ?{}", bind.len()));
            }
            ToBound::Exclusive(end) => {
                bind.push(SqlVal::Text(instant_to_sql(end)?));
                sql.push_str(&format!(" AND at < ?{}", bind.len()));
            }
        }
    }
    if let Some(agent) = q.agent {
        bind.push(SqlVal::Text(agent.to_string()));
        sql.push_str(&format!(" AND agent = ?{}", bind.len()));
    }
    for filter in q.filters {
        match filter {
            Filter::Field { name, value } => {
                bind.push(SqlVal::from_field(value));
                match value {
                    FieldValue::Number(_) => {
                        sql.push_str(&format!(
                            " AND bottle_dec_eq({}, ?{})",
                            quote_ident(name.as_str()),
                            bind.len()
                        ));
                    }
                    _ => {
                        sql.push_str(&format!(
                            " AND {} = ?{}",
                            quote_ident(name.as_str()),
                            bind.len()
                        ));
                    }
                }
            }
            Filter::Link { name, to } => {
                bind.push(SqlVal::Text(q.schema.to_string()));
                bind.push(SqlVal::Text(name.to_string()));
                bind.push(SqlVal::Text(to.schema.to_string()));
                bind.push(SqlVal::Int(to.id));
                let a = bind.len() - 3;
                sql.push_str(&format!(
                    " AND id IN (SELECT from_id FROM links WHERE from_schema = ?{a} AND name = ?{} AND to_schema = ?{} AND to_id = ?{})",
                    a + 1,
                    a + 2,
                    a + 3
                ));
            }
        }
    }
    match q.order {
        Order::Oldest => sql.push_str(" ORDER BY at ASC, id ASC"),
        Order::Newest => sql.push_str(" ORDER BY at DESC, id DESC"),
    }
    if let Some(limit) = q.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }
    let mut entries = {
        let mut stmt = conn.as_ref().prepare(&sql)?;
        let col_count = stmt.column_count();
        let names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
            .collect();
        let mut raw = stmt.query(rusqlite::params_from_iter(
            bind.iter().map(SqlVal::as_param),
        ))?;
        let mut entries = Vec::new();
        while let Some(r) = raw.next()? {
            entries.push(read_entry(q.spec, &names, r)?);
        }
        entries
    };
    attach_links(conn, q.schema, &mut entries)?;
    Ok(entries)
}

fn read_entry(spec: &Spec, names: &[String], r: &rusqlite::Row<'_>) -> Result<Entry, Error> {
    let mut values = HashMap::new();
    let mut id = 0_i64;
    let mut at_raw = String::new();
    let mut agent: Option<String> = None;
    let mut ignored = false;
    for (i, name) in names.iter().enumerate() {
        match name.as_str() {
            "id" => id = r.get(i)?,
            "at" => at_raw = r.get(i)?,
            "agent" => agent = r.get(i)?,
            "ignored" => {
                let v: i64 = r.get(i)?;
                ignored = v != 0;
            }
            other => {
                let Ok(field_name) = FieldName::parse(other) else {
                    continue;
                };
                if let Some(field) = spec.field(&field_name) {
                    let name = field.name.clone();
                    match field.type_ {
                        FieldType::Number => {
                            let v: Option<String> = r.get(i)?;
                            values.insert(
                                name,
                                match v {
                                    Some(s) if !s.is_empty() => FieldValue::Number(s.parse()?),
                                    _ => FieldValue::Empty,
                                },
                            );
                        }
                        FieldType::Enum => {
                            let v: Option<String> = r.get(i)?;
                            values.insert(
                                name,
                                match v {
                                    Some(s) if !s.is_empty() => {
                                        FieldValue::Enum(crate::spec::EnumValue::parse(&s)?)
                                    }
                                    _ => FieldValue::Empty,
                                },
                            );
                        }
                        FieldType::Text => {
                            let v: Option<String> = r.get(i)?;
                            values.insert(
                                name,
                                match v {
                                    Some(s) if !s.is_empty() => FieldValue::Text(s),
                                    _ => FieldValue::Empty,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(Entry {
        id,
        at: instant_from_sql(at_raw)?,
        agent: agent.map(Agent::new),
        ignored,
        values,
        links: Vec::new(),
    })
}

fn attach_links(
    conn: &impl Connection,
    schema: &SchemaName,
    entries: &mut [Entry],
) -> Result<(), Error> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut sql = String::from(
        "SELECT from_id, name, to_schema, to_id FROM links WHERE from_schema = ?1 AND from_id IN (",
    );
    let mut bind = vec![SqlVal::Text(schema.to_string())];
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            sql.push_str(", ");
        }
        bind.push(SqlVal::Int(entry.id));
        sql.push_str(&format!("?{}", bind.len()));
    }
    sql.push_str(") ORDER BY from_id, name");
    let mut stmt = conn.as_ref().prepare(&sql)?;
    let mut raw = stmt.query(rusqlite::params_from_iter(
        bind.iter().map(SqlVal::as_param),
    ))?;
    let mut by_id: HashMap<i64, Vec<Link>> = HashMap::new();
    while let Some(r) = raw.next()? {
        let from_id: i64 = r.get(0)?;
        let name: String = r.get(1)?;
        let to_schema: String = r.get(2)?;
        let to_id: i64 = r.get(3)?;
        let name =
            LinkName::parse(&name).map_err(|_| Error::Fail(Fail::CorruptLinkName(name.clone())))?;
        let to_schema = SchemaName::parse(&to_schema)
            .map_err(|_| Error::Fail(Fail::CorruptLinkSchema(to_schema.clone())))?;
        by_id.entry(from_id).or_default().push(Link {
            name,
            to: EntryRef {
                schema: to_schema,
                id: to_id,
            },
        });
    }
    for entry in entries {
        entry.links = by_id.remove(&entry.id).unwrap_or_default();
    }
    Ok(())
}
