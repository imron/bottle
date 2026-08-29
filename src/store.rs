use std::collections::{BTreeMap, HashMap};

use rusqlite::OptionalExtension;
use rust_decimal::Decimal;

use crate::db::Conn;
use crate::error::{Error, Fail};
use crate::ledger::{
    Agent, Entry, FieldValue, Filter, Find, NonEmptyFieldValue, Order, Schema, SchemaInfo, Summed,
};
use crate::spec::{
    EntryId, EntryRef, EnumValue, Field, FieldKind, FieldName, Group, Link, LinkName, SchemaName,
    Spec, TimePeriod,
};
use crate::sql::{
    SqlVal, StoredAgent, StoredEntryId, StoredEnum, StoredFieldName, StoredLinkName,
    StoredLinkSchema, StoredNumber, StoredSchemaName, StoredText, StoredTime, instant_to_sql,
    quote_ident, table_name,
};
use crate::time::{Instant, Period, ToBound};

pub fn list_schemas(conn: &Conn<'_>) -> Result<Vec<SchemaInfo>, Error> {
    let mut stmt = conn
        .sqlite()
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

pub fn load_schema(conn: &Conn<'_>, name: &SchemaName) -> Result<Schema, Error> {
    let retired: Option<i64> = conn
        .sqlite()
        .query_row(
            "SELECT retired FROM schemas WHERE name = ?1",
            [name.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    let Some(retired) = retired else {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    };
    Ok(Schema {
        spec: load_spec(conn, name)?,
        retired: retired != 0,
    })
}

fn load_spec(conn: &Conn<'_>, schema: &SchemaName) -> Result<Spec, Error> {
    let rows = {
        let mut stmt = conn.sqlite().prepare(
            "SELECT name, kind, required FROM schema_fields WHERE schema = ?1 ORDER BY position",
        )?;
        let mut raw = stmt.query([schema.as_str()])?;
        let mut rows = Vec::new();
        while let Some(r) = raw.next()? {
            let name: String = r.get(0)?;
            let kind: String = r.get(1)?;
            let required: i64 = r.get(2)?;
            rows.push((name, kind, required));
        }
        rows
    };
    let mut fields = Vec::new();
    for (name, kind, required) in rows {
        let name = FieldName::try_from(StoredFieldName(name))?;
        fields.push(Field {
            kind: load_kind(conn, schema, &name, &kind)?,
            name,
            required: required != 0,
        });
    }
    Ok(Spec { fields })
}

fn load_kind(
    conn: &Conn<'_>,
    schema: &SchemaName,
    name: &FieldName,
    kind: &str,
) -> Result<FieldKind, Error> {
    match kind {
        "text" => Ok(FieldKind::Text),
        "number" => Ok(FieldKind::Number),
        "enum" => {
            let values = load_enum_values(conn, schema, name)?;
            if values.is_empty() {
                return Err(Error::Fail(Fail::CorruptStoredFieldKind(
                    name.as_str().to_string(),
                )));
            }
            Ok(FieldKind::Enum(values))
        }
        other => Err(Error::Fail(Fail::CorruptStoredFieldKind(other.to_string()))),
    }
}

fn load_enum_values(
    conn: &Conn<'_>,
    schema: &SchemaName,
    field: &FieldName,
) -> Result<Vec<EnumValue>, Error> {
    let mut stmt = conn.sqlite().prepare(
        "SELECT value FROM schema_enum_values
         WHERE schema = ?1 AND field = ?2
         ORDER BY position",
    )?;
    let mut raw = stmt.query(rusqlite::params![schema.as_str(), field.as_str()])?;
    let mut values = Vec::new();
    while let Some(r) = raw.next()? {
        let value: String = r.get(0)?;
        values.push(EnumValue::try_from(StoredEnum(value))?);
    }
    Ok(values)
}

fn require_schema(conn: &Conn<'_>, name: &SchemaName) -> Result<(), Error> {
    let found: Option<i64> = conn
        .sqlite()
        .query_row(
            "SELECT 1 FROM schemas WHERE name = ?1",
            [name.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    if found.is_none() {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    }
    Ok(())
}

pub fn entry_exists(conn: &Conn<'_>, schema: &SchemaName, id: EntryId) -> Result<bool, Error> {
    require_schema(conn, schema)?;
    let sql = format!(
        "SELECT 1 FROM {} WHERE id = ?1",
        quote_ident(&table_name(schema))
    );
    let found: Option<i64> = conn
        .sqlite()
        .query_row(&sql, [id.as_i64()], |row| row.get(0))
        .optional()?;
    Ok(found.is_some())
}

pub fn entry_at(
    conn: &Conn<'_>,
    schema: &SchemaName,
    id: EntryId,
) -> Result<Option<Instant>, Error> {
    require_schema(conn, schema)?;
    let sql = format!(
        "SELECT at FROM {} WHERE id = ?1",
        quote_ident(&table_name(schema))
    );
    let raw: Option<String> = conn
        .sqlite()
        .query_row(&sql, [id.as_i64()], |row| row.get(0))
        .optional()?;
    match raw {
        None => Ok(None),
        Some(raw) => Ok(Some(Instant::try_from(StoredTime(raw))?)),
    }
}

pub fn inbound_link_count(conn: &Conn<'_>, name: &SchemaName) -> Result<i64, Error> {
    let n: i64 = conn.sqlite().query_row(
        "SELECT COUNT(*) FROM links WHERE to_schema = ?1",
        [name.as_str()],
        |row| row.get(0),
    )?;
    Ok(n)
}

pub fn has_outbound_link_name(
    conn: &Conn<'_>,
    schema: &SchemaName,
    name: &LinkName,
) -> Result<bool, Error> {
    let found: Option<i64> = conn
        .sqlite()
        .query_row(
            "SELECT 1 FROM links WHERE from_schema = ?1 AND name = ?2 LIMIT 1",
            rusqlite::params![schema.as_str(), name.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

pub fn get_entry(
    conn: &Conn<'_>,
    schema: &SchemaName,
    spec: &Spec,
    id: EntryId,
) -> Result<Option<Entry>, Error> {
    let sql = format!(
        "SELECT {} FROM {} WHERE id = ?1",
        entry_columns(spec),
        quote_ident(&table_name(schema))
    );
    let mut entry = {
        let mut stmt = conn.sqlite().prepare(&sql)?;
        let mut raw = stmt.query([id.as_i64()])?;
        match raw.next()? {
            Some(r) => read_entry(spec, r)?,
            None => return Ok(None),
        }
    };
    entry.links = links_for(conn, schema, id)?;
    Ok(Some(entry))
}

pub fn find(conn: &Conn<'_>, q: Find<'_>) -> Result<Vec<Entry>, Error> {
    let mut sql = format!(
        "SELECT {} FROM {} WHERE 1=1",
        entry_columns(q.spec),
        quote_ident(&table_name(q.schema))
    );
    let mut bind: Vec<SqlVal> = Vec::new();
    apply_find_filters(&mut sql, &mut bind, &q)?;
    apply_find_order_limit(&mut sql, &q);
    let mut entries = {
        let mut stmt = conn.sqlite().prepare(&sql)?;
        let mut raw = stmt.query(rusqlite::params_from_iter(
            bind.iter().map(SqlVal::as_param),
        ))?;
        let mut entries = Vec::new();
        while let Some(r) = raw.next()? {
            entries.push(read_entry(q.spec, r)?);
        }
        entries
    };
    attach_links(conn, &q, &mut entries)?;
    Ok(entries)
}

pub fn sum(
    conn: &Conn<'_>,
    q: Find<'_>,
    field: &FieldName,
    group: Option<Group>,
) -> Result<Summed, Error> {
    match group {
        None => total(conn, q, field),
        Some(Group::Time(unit)) => by_time(conn, q, field, unit),
        Some(Group::Link(name)) => by_link(conn, q, field, name),
    }
}

fn total(conn: &Conn<'_>, q: Find<'_>, field: &FieldName) -> Result<Summed, Error> {
    let col = quote_ident(field.as_str());
    let mut sql = format!(
        "SELECT bottle_dec_sum({col}) FROM {} WHERE 1=1",
        quote_ident(&table_name(q.schema))
    );
    let mut bind = Vec::new();
    apply_find_filters(&mut sql, &mut bind, &q)?;
    let raw: String = conn.sqlite().query_row(
        &sql,
        rusqlite::params_from_iter(bind.iter().map(SqlVal::as_param)),
        |row| row.get(0),
    )?;
    Ok(Summed::Total(Decimal::try_from(StoredNumber(raw))?))
}

fn by_time(
    conn: &Conn<'_>,
    q: Find<'_>,
    field: &FieldName,
    unit: TimePeriod,
) -> Result<Summed, Error> {
    let col = quote_ident(field.as_str());
    let mut sql = format!(
        "SELECT bottle_period(at, ?1), bottle_dec_sum({col}) FROM {} WHERE 1=1",
        quote_ident(&table_name(q.schema))
    );
    let mut bind = vec![SqlVal::Text(unit.as_str().to_string())];
    apply_find_filters(&mut sql, &mut bind, &q)?;
    sql.push_str(&format!(
        " AND {col} IS NOT NULL AND {col} != '' GROUP BY 1"
    ));
    let mut stmt = conn.sqlite().prepare(&sql)?;
    let mut raw = stmt.query(rusqlite::params_from_iter(
        bind.iter().map(SqlVal::as_param),
    ))?;
    let mut buckets: BTreeMap<Period, Decimal> = BTreeMap::new();
    while let Some(r) = raw.next()? {
        let key: String = r.get(0)?;
        let total: String = r.get(1)?;
        buckets.insert(
            Period::parse(unit, &key)?,
            Decimal::try_from(StoredNumber(total))?,
        );
    }
    Ok(Summed::Time {
        unit,
        buckets: buckets.into_iter().collect(),
    })
}

fn by_link(
    conn: &Conn<'_>,
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
    let mut stmt = conn.sqlite().prepare(&sql)?;
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
                Some(EntryRef {
                    schema,
                    id: EntryId::try_from(StoredEntryId(id))?,
                })
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
                bind.push(SqlVal::from_filter(value));
                match value {
                    NonEmptyFieldValue::Number(_) => {
                        sql.push_str(&format!(
                            " AND bottle_dec_eq({}, ?{})",
                            quote_ident(name.as_str()),
                            bind.len()
                        ));
                    }
                    NonEmptyFieldValue::Text(_) | NonEmptyFieldValue::Enum(_) => {
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
                bind.push(SqlVal::Int(to.id.as_i64()));
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

fn entry_columns(spec: &Spec) -> String {
    let mut cols = vec![
        "id".to_string(),
        "at".to_string(),
        "agent".to_string(),
        "ignored".to_string(),
    ];
    for field in &spec.fields {
        cols.push(quote_ident(field.name.as_str()));
    }
    cols.join(", ")
}

fn read_entry(spec: &Spec, r: &rusqlite::Row<'_>) -> Result<Entry, Error> {
    let id: i64 = r.get(0)?;
    let at_raw: String = r.get(1)?;
    let agent: Option<String> = r.get(2)?;
    let ignored: i64 = r.get(3)?;
    let mut values = HashMap::new();
    for (i, field) in spec.fields.iter().enumerate() {
        values.insert(field.name.clone(), read_field_value(field, r, 4 + i)?);
    }
    Ok(Entry {
        id: EntryId::try_from(StoredEntryId(id))?,
        at: Instant::try_from(StoredTime(at_raw))?,
        agent: agent.map(StoredAgent).map(Agent::try_from).transpose()?,
        ignored: ignored != 0,
        values,
        links: Vec::new(),
    })
}

fn read_field_value(field: &Field, r: &rusqlite::Row<'_>, i: usize) -> Result<FieldValue, Error> {
    let v: Option<String> = r.get(i)?;
    Ok(match &field.kind {
        FieldKind::Number => match v {
            Some(s) if !s.is_empty() => FieldValue::Number(Decimal::try_from(StoredNumber(s))?),
            _ => FieldValue::Empty,
        },
        FieldKind::Enum(_) => match v {
            Some(s) if !s.is_empty() => FieldValue::Enum(EnumValue::try_from(StoredEnum(s))?),
            _ => FieldValue::Empty,
        },
        FieldKind::Text => match v {
            Some(s) if !s.is_empty() => FieldValue::Text(String::try_from(StoredText(s))?),
            _ => FieldValue::Empty,
        },
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

fn links_for(conn: &Conn<'_>, schema: &SchemaName, id: EntryId) -> Result<Vec<Link>, Error> {
    let mut stmt = conn.sqlite().prepare(
        "SELECT from_id, name, to_schema, to_id FROM links
         WHERE from_schema = ?1 AND from_id = ?2
         ORDER BY name",
    )?;
    let mut raw = stmt.query(rusqlite::params![schema.as_str(), id.as_i64()])?;
    let mut links = Vec::new();
    while let Some(r) = raw.next()? {
        links.push(read_link(r)?.1);
    }
    Ok(links)
}

fn attach_links(conn: &Conn<'_>, q: &Find<'_>, entries: &mut [Entry]) -> Result<(), Error> {
    if entries.is_empty() {
        return Ok(());
    }
    // Re-apply the same filter as a CTE. Binding matched ids as
    // `from_id IN (?,?,…)` dies at SQLITE_MAX_VARIABLE_NUMBER (32766).
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
    let mut stmt = conn.sqlite().prepare(&sql)?;
    let mut raw = stmt.query(rusqlite::params_from_iter(
        bind.iter().map(SqlVal::as_param),
    ))?;
    let mut by_id: HashMap<EntryId, Vec<Link>> = HashMap::new();
    while let Some(r) = raw.next()? {
        let (from_id, link) = read_link(r)?;
        by_id.entry(from_id).or_default().push(link);
    }
    for entry in entries {
        entry.links = by_id.remove(&entry.id).unwrap_or_default();
    }
    Ok(())
}

fn read_link(r: &rusqlite::Row<'_>) -> Result<(EntryId, Link), Error> {
    let from_id: i64 = r.get(0)?;
    let name: String = r.get(1)?;
    let to_schema: String = r.get(2)?;
    let to_id: i64 = r.get(3)?;
    Ok((
        EntryId::try_from(StoredEntryId(from_id))?,
        Link {
            name: LinkName::try_from(StoredLinkName(name))?,
            to: EntryRef {
                schema: SchemaName::try_from(StoredLinkSchema(to_schema))?,
                id: EntryId::try_from(StoredEntryId(to_id))?,
            },
        },
    ))
}
