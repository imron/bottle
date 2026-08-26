use std::collections::{BTreeMap, HashMap};

use rusqlite::OptionalExtension;
use rust_decimal::Decimal;

use crate::db::Connection;
use crate::error::{Error, Fail};
use crate::ledger::{Agent, Entry, FieldValue, Filter, Order, Schema, SchemaInfo};
use crate::spec::{
    EntryRef, EnumValue, FieldKind, FieldName, Group, Link, LinkName, SchemaName, Spec, TimePeriod,
};
use crate::sql::{
    SqlVal, StoredAgent, StoredEnum, StoredLinkName, StoredLinkSchema, StoredNumber,
    StoredSchemaName, StoredTime, instant_to_sql, quote_ident, table_name,
};
use crate::time::{Instant, Period, Range, ToBound, period};
use jiff::tz::TimeZone;

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
        let name = SchemaName::try_from(StoredSchemaName(name))?;
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
    entry.links = links_for(conn, schema, id)?;
    Ok(Some(entry))
}

pub fn find(conn: &impl Connection, q: Find<'_>) -> Result<Vec<Entry>, Error> {
    let mut sql = format!(
        "SELECT * FROM {} WHERE 1=1",
        quote_ident(&table_name(q.schema))
    );
    let mut bind: Vec<SqlVal> = Vec::new();
    apply_find_filters(&mut sql, &mut bind, &q)?;
    apply_find_order_limit(&mut sql, &q);
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
    attach_links(conn, &q, &mut entries)?;
    Ok(entries)
}

pub enum Summed {
    Total(Decimal),
    Time {
        unit: TimePeriod,
        buckets: Vec<(Period, Decimal)>,
    },
    Link {
        name: LinkName,
        buckets: Vec<(Option<EntryRef>, Decimal)>,
    },
}

pub fn sum(
    conn: &impl Connection,
    q: Find<'_>,
    field: &FieldName,
    group: Option<Group>,
    tz: &TimeZone,
) -> Result<Summed, Error> {
    match group {
        None => total(conn, q, field),
        Some(Group::Time(unit)) => by_time(conn, q, field, unit, tz),
        Some(Group::Link(name)) => by_link(conn, q, field, name),
    }
}

fn total(conn: &impl Connection, q: Find<'_>, field: &FieldName) -> Result<Summed, Error> {
    let col = quote_ident(field.as_str());
    let mut sql = format!(
        "SELECT bottle_dec_sum({col}) FROM {} WHERE 1=1",
        quote_ident(&table_name(q.schema))
    );
    let mut bind = Vec::new();
    apply_find_filters(&mut sql, &mut bind, &q)?;
    let raw: String = conn.as_ref().query_row(
        &sql,
        rusqlite::params_from_iter(bind.iter().map(SqlVal::as_param)),
        |row| row.get(0),
    )?;
    Ok(Summed::Total(Decimal::try_from(StoredNumber(raw))?))
}

fn by_time(
    conn: &impl Connection,
    q: Find<'_>,
    field: &FieldName,
    unit: TimePeriod,
    tz: &TimeZone,
) -> Result<Summed, Error> {
    let col = quote_ident(field.as_str());
    let mut sql = format!(
        "SELECT at, {col} FROM {} WHERE 1=1",
        quote_ident(&table_name(q.schema))
    );
    let mut bind = Vec::new();
    apply_find_filters(&mut sql, &mut bind, &q)?;
    sql.push_str(&format!(" AND {col} IS NOT NULL AND {col} != ''"));
    let mut stmt = conn.as_ref().prepare(&sql)?;
    let mut raw = stmt.query(rusqlite::params_from_iter(
        bind.iter().map(SqlVal::as_param),
    ))?;
    let mut buckets: BTreeMap<Period, Decimal> = BTreeMap::new();
    while let Some(r) = raw.next()? {
        let at_raw: String = r.get(0)?;
        let n_raw: String = r.get(1)?;
        let n = Decimal::try_from(StoredNumber(n_raw))?;
        let k = period(unit, Instant::try_from(StoredTime(at_raw))?, tz);
        *buckets.entry(k).or_insert(Decimal::ZERO) += n;
    }
    Ok(Summed::Time {
        unit,
        buckets: buckets.into_iter().collect(),
    })
}

fn by_link(
    conn: &impl Connection,
    q: Find<'_>,
    field: &FieldName,
    name: LinkName,
) -> Result<Summed, Error> {
    let col = quote_ident(field.as_str());
    let table = quote_ident(&table_name(q.schema));
    let mut inner = format!("SELECT id, {col} AS n FROM {table} WHERE 1=1");
    let mut bind = Vec::new();
    apply_find_filters(&mut inner, &mut bind, &q)?;
    inner.push_str(&format!(" AND {col} IS NOT NULL AND {col} != ''"));
    bind.push(SqlVal::Text(q.schema.to_string()));
    bind.push(SqlVal::Text(name.to_string()));
    let a = bind.len() - 1;
    let sql = format!(
        "SELECT l.to_schema, l.to_id, bottle_dec_sum(x.n) FROM ({inner}) x
         LEFT JOIN links l ON l.from_schema = ?{a} AND l.from_id = x.id AND l.name = ?{}
         GROUP BY l.to_schema, l.to_id",
        a + 1
    );
    let mut stmt = conn.as_ref().prepare(&sql)?;
    let mut raw = stmt.query(rusqlite::params_from_iter(
        bind.iter().map(SqlVal::as_param),
    ))?;
    let mut buckets: BTreeMap<Option<EntryRef>, Decimal> = BTreeMap::new();
    while let Some(r) = raw.next()? {
        let to_schema: Option<String> = r.get(0)?;
        let to_id: Option<i64> = r.get(1)?;
        let total: String = r.get(2)?;
        let key = match (to_schema, to_id) {
            (Some(schema), Some(id)) => {
                let schema = SchemaName::try_from(StoredLinkSchema(schema))?;
                Some(EntryRef { schema, id })
            }
            _ => None,
        };
        buckets.insert(key, Decimal::try_from(StoredNumber(total))?);
    }
    Ok(Summed::Link {
        name,
        buckets: buckets.into_iter().collect(),
    })
}

fn apply_find_filters(sql: &mut String, bind: &mut Vec<SqlVal>, q: &Find<'_>) -> Result<(), Error> {
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
    Ok(())
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
                    match &field.kind {
                        FieldKind::Number => {
                            let v: Option<String> = r.get(i)?;
                            values.insert(
                                name,
                                match v {
                                    Some(s) if !s.is_empty() => {
                                        FieldValue::Number(Decimal::try_from(StoredNumber(s))?)
                                    }
                                    _ => FieldValue::Empty,
                                },
                            );
                        }
                        FieldKind::Enum(_) => {
                            let v: Option<String> = r.get(i)?;
                            values.insert(
                                name,
                                match v {
                                    Some(s) if !s.is_empty() => {
                                        FieldValue::Enum(EnumValue::try_from(StoredEnum(s))?)
                                    }
                                    _ => FieldValue::Empty,
                                },
                            );
                        }
                        FieldKind::Text => {
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
        at: Instant::try_from(StoredTime(at_raw))?,
        agent: agent.map(StoredAgent).map(Agent::try_from).transpose()?,
        ignored,
        values,
        links: Vec::new(),
    })
}

fn apply_find_order_limit(sql: &mut String, q: &Find<'_>) {
    match q.order {
        Order::Oldest => sql.push_str(" ORDER BY at ASC, id ASC"),
        Order::Newest => sql.push_str(" ORDER BY at DESC, id DESC"),
    }
    if let Some(limit) = q.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }
}

fn links_for(conn: &impl Connection, schema: &SchemaName, id: i64) -> Result<Vec<Link>, Error> {
    let mut stmt = conn.as_ref().prepare(
        "SELECT from_id, name, to_schema, to_id FROM links
         WHERE from_schema = ?1 AND from_id = ?2
         ORDER BY name",
    )?;
    let mut raw = stmt.query(rusqlite::params![schema.as_str(), id])?;
    let mut links = Vec::new();
    while let Some(r) = raw.next()? {
        links.push(read_link(r)?.1);
    }
    Ok(links)
}

fn attach_links(conn: &impl Connection, q: &Find<'_>, entries: &mut [Entry]) -> Result<(), Error> {
    if entries.is_empty() {
        return Ok(());
    }
    let table = quote_ident(&table_name(q.schema));
    let mut matched = format!("SELECT id FROM {table} WHERE 1=1");
    let mut bind = Vec::new();
    apply_find_filters(&mut matched, &mut bind, q)?;
    apply_find_order_limit(&mut matched, q);
    bind.push(SqlVal::Text(q.schema.to_string()));
    let schema_at = bind.len();
    let sql = format!(
        "WITH matched AS ({matched})
         SELECT l.from_id, l.name, l.to_schema, l.to_id
         FROM links l
         JOIN matched m ON m.id = l.from_id
         WHERE l.from_schema = ?{schema_at}
         ORDER BY l.from_id, l.name"
    );
    let mut stmt = conn.as_ref().prepare(&sql)?;
    let mut raw = stmt.query(rusqlite::params_from_iter(
        bind.iter().map(SqlVal::as_param),
    ))?;
    let mut by_id: HashMap<i64, Vec<Link>> = HashMap::new();
    while let Some(r) = raw.next()? {
        let (from_id, link) = read_link(r)?;
        by_id.entry(from_id).or_default().push(link);
    }
    for entry in entries {
        entry.links = by_id.remove(&entry.id).unwrap_or_default();
    }
    Ok(())
}

fn read_link(r: &rusqlite::Row<'_>) -> Result<(i64, Link), Error> {
    let from_id: i64 = r.get(0)?;
    let name: String = r.get(1)?;
    let to_schema: String = r.get(2)?;
    let to_id: i64 = r.get(3)?;
    Ok((
        from_id,
        Link {
            name: LinkName::try_from(StoredLinkName(name))?,
            to: EntryRef {
                schema: SchemaName::try_from(StoredLinkSchema(to_schema))?,
                id: to_id,
            },
        },
    ))
}
