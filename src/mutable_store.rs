use std::collections::HashMap;

use rusqlite::params;

use crate::db::{Tx, UniqueConstraint};
use crate::error::{Error, Fail};
use crate::ledger::{Agent, FieldValue, NonEmptyFieldValue};
use crate::spec::{
    EntryId, EnumValue, Field, FieldKind, FieldName, Link, LinkName, SchemaName, Spec,
};
use crate::sql::{SqlVal, StoredEntryId, instant_to_sql, quote_ident, sql_default, table_name};
use crate::time::Instant;

pub fn insert_schema(tx: &mut Tx<'_>, name: &SchemaName, spec: &Spec) -> Result<(), Error> {
    let cols = create_columns(spec);
    let sql = format!("CREATE TABLE {} ({cols})", quote_ident(&table_name(name)));
    tx.sqlite()
        .execute(
            "INSERT INTO schemas (name, retired) VALUES (?1, 0)",
            params![name.as_str()],
        )
        .unique(Fail::SchemaExists(name.clone()))?;
    for (i, field) in spec.fields.iter().enumerate() {
        insert_field(tx, name, field, i as i64)?;
    }
    tx.sqlite().execute_batch(&sql)?;
    Ok(())
}

pub fn add_column(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    field: &Field,
    default: Option<&NonEmptyFieldValue>,
) -> Result<(), Error> {
    let table = quote_ident(&table_name(schema));
    let alter = if let Some(def) = default {
        let sql_def = sql_default(def);
        format!(
            "ALTER TABLE {table} ADD COLUMN {} TEXT NOT NULL DEFAULT {sql_def}",
            quote_ident(field.name.as_str())
        )
    } else {
        format!(
            "ALTER TABLE {table} ADD COLUMN {} TEXT",
            quote_ident(field.name.as_str())
        )
    };
    tx.sqlite().execute_batch(&alter)?;
    Ok(())
}

pub fn insert_field(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    field: &Field,
    position: i64,
) -> Result<(), Error> {
    let kind = match &field.kind {
        FieldKind::Text => "text",
        FieldKind::Number => "number",
        FieldKind::Enum(_) => "enum",
    };
    tx.sqlite().execute(
        "INSERT INTO schema_fields (schema, position, name, kind, required)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            schema.as_str(),
            position,
            field.name.as_str(),
            kind,
            field.required as i64
        ],
    )?;
    if let FieldKind::Enum(values) = &field.kind {
        for (i, value) in values.iter().enumerate() {
            tx.sqlite().execute(
                "INSERT INTO schema_enum_values (schema, field, position, value)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    schema.as_str(),
                    field.name.as_str(),
                    i as i64,
                    value.as_str()
                ],
            )?;
        }
    }
    Ok(())
}

pub fn insert_enum_value(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    field: &FieldName,
    value: &EnumValue,
    position: i64,
) -> Result<(), Error> {
    tx.sqlite()
        .execute(
            "INSERT INTO schema_enum_values (schema, field, position, value)
             VALUES (?1, ?2, ?3, ?4)",
            params![schema.as_str(), field.as_str(), position, value.as_str()],
        )
        .unique(Fail::EnumValueExists(value.clone()))?;
    Ok(())
}

pub fn retire(tx: &mut Tx<'_>, name: &SchemaName) -> Result<(), Error> {
    let n = tx.sqlite().execute(
        "UPDATE schemas SET retired = 1 WHERE name = ?1",
        [name.as_str()],
    )?;
    if n == 0 {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    }
    Ok(())
}

pub fn drop_schema(tx: &mut Tx<'_>, name: &SchemaName) -> Result<(), Error> {
    tx.sqlite().execute(
        "DELETE FROM schema_enum_values WHERE schema = ?1",
        [name.as_str()],
    )?;
    tx.sqlite().execute(
        "DELETE FROM schema_fields WHERE schema = ?1",
        [name.as_str()],
    )?;
    let n = tx
        .sqlite()
        .execute("DELETE FROM schemas WHERE name = ?1", [name.as_str()])?;
    if n == 0 {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    }
    tx.sqlite()
        .execute("DELETE FROM links WHERE from_schema = ?1", [name.as_str()])?;
    tx.sqlite()
        .execute_batch(&format!("DROP TABLE {}", quote_ident(&table_name(name))))?;
    Ok(())
}

pub fn insert_entry(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    spec: &Spec,
    at: Instant,
    agent: Option<&Agent>,
    values: &HashMap<FieldName, FieldValue>,
    links: &[Link],
) -> Result<EntryId, Error> {
    let mut col_names = vec!["at".to_string(), "agent".to_string()];
    let mut placeholders = vec!["?1".to_string(), "?2".to_string()];
    let mut bind: Vec<SqlVal> = vec![
        SqlVal::Text(instant_to_sql(at)?),
        match agent {
            Some(a) => SqlVal::Text(a.to_string()),
            None => SqlVal::Null,
        },
    ];
    for field in &spec.fields {
        col_names.push(field.name.to_string());
        placeholders.push(format!("?{}", bind.len() + 1));
        bind.push(
            values
                .get(&field.name)
                .map(SqlVal::from_field)
                .unwrap_or(SqlVal::Null),
        );
    }
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        quote_ident(&table_name(schema)),
        col_names
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", "),
        placeholders.join(", ")
    );
    {
        let mut stmt = tx.sqlite().prepare(&sql)?;
        stmt.execute(rusqlite::params_from_iter(
            bind.iter().map(SqlVal::as_param),
        ))?;
    }
    let id = EntryId::try_from(StoredEntryId(tx.sqlite().last_insert_rowid()))?;
    insert_links(tx, schema, id, links)?;
    Ok(id)
}

pub fn update_entry(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: EntryId,
    at: Option<Instant>,
    agent: Option<&Agent>,
    values: &HashMap<FieldName, FieldValue>,
) -> Result<(), Error> {
    let mut sets = Vec::new();
    let mut bind: Vec<SqlVal> = Vec::new();
    if let Some(at) = at {
        sets.push(format!("at = ?{}", bind.len() + 1));
        bind.push(SqlVal::Text(instant_to_sql(at)?));
    }
    if let Some(agent) = agent {
        sets.push(format!("agent = ?{}", bind.len() + 1));
        bind.push(SqlVal::Text(agent.to_string()));
    }
    for (name, val) in values {
        sets.push(format!(
            "{} = ?{}",
            quote_ident(name.as_str()),
            bind.len() + 1
        ));
        bind.push(SqlVal::from_field(val));
    }
    if sets.is_empty() {
        return Ok(());
    }
    bind.push(SqlVal::Int(id.as_i64()));
    let sql = format!(
        "UPDATE {} SET {} WHERE id = ?{}",
        quote_ident(&table_name(schema)),
        sets.join(", "),
        bind.len()
    );
    tx.sqlite().execute(
        &sql,
        rusqlite::params_from_iter(bind.iter().map(SqlVal::as_param)),
    )?;
    Ok(())
}

pub fn delete_link(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: EntryId,
    name: &LinkName,
) -> Result<(), Error> {
    tx.sqlite().execute(
        "DELETE FROM links WHERE from_schema = ?1 AND from_id = ?2 AND name = ?3",
        params![schema.as_str(), id.as_i64(), name.as_str()],
    )?;
    Ok(())
}

pub fn upsert_link(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: EntryId,
    link: &Link,
) -> Result<(), Error> {
    tx.sqlite().execute(
        "INSERT INTO links (from_schema, from_id, name, to_schema, to_id)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (from_schema, from_id, name) DO UPDATE SET
            to_schema = excluded.to_schema,
            to_id = excluded.to_id",
        params![
            schema.as_str(),
            id.as_i64(),
            link.name.as_str(),
            link.to.schema.as_str(),
            link.to.id.as_i64()
        ],
    )?;
    Ok(())
}

pub fn set_ignored(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: EntryId,
    ignored: bool,
) -> Result<(), Error> {
    tx.sqlite().execute(
        &format!(
            "UPDATE {} SET ignored = ?1 WHERE id = ?2",
            quote_ident(&table_name(schema))
        ),
        params![ignored as i64, id.as_i64()],
    )?;
    Ok(())
}

fn insert_links(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: EntryId,
    links: &[Link],
) -> Result<(), Error> {
    for link in links {
        tx.sqlite().execute(
            "INSERT INTO links (from_schema, from_id, name, to_schema, to_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                schema.as_str(),
                id.as_i64(),
                link.name.as_str(),
                link.to.schema.as_str(),
                link.to.id.as_i64()
            ],
        )?;
    }
    Ok(())
}

fn create_columns(spec: &Spec) -> String {
    let mut cols = vec![
        "id INTEGER PRIMARY KEY".to_string(),
        "at TEXT NOT NULL".to_string(),
        "agent TEXT".to_string(),
        "ignored INTEGER NOT NULL DEFAULT 0".to_string(),
    ];
    for field in &spec.fields {
        let mut col = format!("{} TEXT", quote_ident(field.name.as_str()));
        if field.required {
            col.push_str(" NOT NULL");
        }
        cols.push(col);
    }
    cols.join(", ")
}
