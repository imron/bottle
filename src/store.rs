use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Error;
use crate::ledger::{Entry, FieldValue, Filter, Order, Schema, SchemaInfo};
use crate::spec::{EntryRef, FieldType, Link, LinkName, SchemaName, Spec};
use crate::sql::{SqlVal, instant_from_sql, instant_to_sql, quote_ident, table_name};
use crate::time::{Range, ToBound};

pub struct Store {
    conn: Connection,
}

pub(crate) struct Find<'a> {
    pub schema: &'a SchemaName,
    pub spec: &'a Spec,
    pub range: Range,
    pub agent: Option<&'a str>,
    pub include_ignored: bool,
    pub filters: &'a [Filter],
    pub order: Order,
    pub limit: Option<usize>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: crate::db::open(path)?,
        })
    }

    pub(crate) fn schema_exists(&self, name: &SchemaName) -> Result<bool, Error> {
        schema_exists(&self.conn, name)
    }

    pub(crate) fn transaction<T>(
        &mut self,
        f: impl FnOnce(&mut crate::mutable_store::Tx<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut tx = crate::mutable_store::Tx::begin(&mut self.conn)?;
        match f(&mut tx) {
            Ok(value) => {
                tx.commit()?;
                Ok(value)
            }
            Err(err) => Err(err),
        }
    }

    pub(crate) fn list_schemas(&self) -> Result<Vec<SchemaInfo>, Error> {
        list_schemas(&self.conn)
    }

    pub(crate) fn load_schema(&self, name: &SchemaName) -> Result<Schema, Error> {
        load_schema(&self.conn, name)
    }

    pub(crate) fn get_entry(
        &self,
        schema: &SchemaName,
        spec: &Spec,
        id: i64,
    ) -> Result<Option<Entry>, Error> {
        load_entry(&self.conn, schema, spec, id)
    }

    pub(crate) fn find(&self, q: Find<'_>) -> Result<Vec<Entry>, Error> {
        execute_select(&self.conn, &q)
    }
}

pub(crate) fn list_schemas(conn: &Connection) -> Result<Vec<SchemaInfo>, Error> {
    let mut stmt = conn.prepare("SELECT name, retired FROM schemas ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let retired: i64 = row.get(1)?;
        Ok((name, retired))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (name, retired) = row?;
        let name = SchemaName::parse(&name)
            .map_err(|_| Error::fail(format!("corrupt schema name: {name}")))?;
        out.push(SchemaInfo {
            name,
            retired: retired != 0,
        });
    }
    Ok(out)
}

pub(crate) fn load_schema(conn: &Connection, name: &SchemaName) -> Result<Schema, Error> {
    let row = conn
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
        return Err(Error::fail(format!("unknown schema: {name}")));
    };
    Ok(Schema {
        spec: Spec::parse_yaml(&spec)?,
        retired: retired != 0,
    })
}

pub(crate) fn schema_exists(conn: &Connection, name: &SchemaName) -> Result<bool, Error> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM schemas WHERE name = ?1",
        [name.as_str()],
        |row| row.get(0),
    )?;
    Ok(n > 0)
}

pub(crate) fn inbound_link_count(conn: &Connection, name: &SchemaName) -> Result<i64, Error> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM links WHERE to_schema = ?1",
        [name.as_str()],
        |row| row.get(0),
    )?;
    Ok(n)
}

pub(crate) fn load_entry(
    conn: &Connection,
    schema: &SchemaName,
    spec: &Spec,
    id: i64,
) -> Result<Option<Entry>, Error> {
    let table = table_name(schema);
    let sql = format!("SELECT * FROM {} WHERE id = ?1", quote_ident(&table));
    let mut stmt = conn.prepare(&sql)?;
    let col_count = stmt.column_count();
    let names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();
    let mut raw = stmt.query([id])?;
    match raw.next()? {
        Some(r) => Ok(Some(read_entry(conn, schema, spec, &names, r)?)),
        None => Ok(None),
    }
}

fn execute_select(conn: &Connection, q: &Find<'_>) -> Result<Vec<Entry>, Error> {
    let table = table_name(q.schema);
    let mut sql = format!("SELECT * FROM {} WHERE 1=1", quote_ident(&table));
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
                bind.push(SqlVal::Text(q.schema.as_str().to_string()));
                bind.push(SqlVal::Text(name.as_str().to_string()));
                bind.push(SqlVal::Text(to.schema.as_str().to_string()));
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
    let mut stmt = conn.prepare(&sql)?;
    let col_count = stmt.column_count();
    let names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();
    let mut raw = stmt.query(rusqlite::params_from_iter(
        bind.iter().map(SqlVal::as_param),
    ))?;
    let mut entries = Vec::new();
    while let Some(r) = raw.next()? {
        entries.push(read_entry(conn, q.schema, q.spec, &names, r)?);
    }
    Ok(entries)
}

fn read_entry(
    conn: &Connection,
    schema: &SchemaName,
    spec: &Spec,
    names: &[String],
    r: &rusqlite::Row<'_>,
) -> Result<Entry, Error> {
    let mut values = HashMap::new();
    let mut id = 0_i64;
    let mut at_raw = String::new();
    let mut agent = None;
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
                if let Some(field) = spec.field(other) {
                    match field.type_ {
                        FieldType::Number => {
                            let v: Option<String> = r.get(i)?;
                            values.insert(
                                other.to_string(),
                                match v {
                                    Some(s) if !s.is_empty() => FieldValue::Number(s.parse()?),
                                    _ => FieldValue::Empty,
                                },
                            );
                        }
                        _ => {
                            let v: Option<String> = r.get(i)?;
                            values.insert(
                                other.to_string(),
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
        agent,
        ignored,
        values,
        links: load_links(conn, schema, id)?,
    })
}

pub(crate) fn load_links(
    conn: &Connection,
    schema: &SchemaName,
    id: i64,
) -> Result<Vec<Link>, Error> {
    let mut stmt = conn.prepare(
        "SELECT name, to_schema, to_id FROM links
         WHERE from_schema = ?1 AND from_id = ?2
         ORDER BY name",
    )?;
    let rows = stmt.query_map(params![schema.as_str(), id], |row| {
        let name: String = row.get(0)?;
        let to_schema: String = row.get(1)?;
        let to_id: i64 = row.get(2)?;
        Ok((name, to_schema, to_id))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (name, to_schema, to_id) = row?;
        let name = LinkName::parse(&name)
            .map_err(|_| Error::fail(format!("corrupt link name: {name}")))?;
        let to_schema = SchemaName::parse(&to_schema)
            .map_err(|_| Error::fail(format!("corrupt link schema: {to_schema}")))?;
        out.push(Link {
            name,
            to: EntryRef {
                schema: to_schema,
                id: to_id,
            },
        });
    }
    Ok(out)
}
