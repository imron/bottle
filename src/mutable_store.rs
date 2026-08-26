use std::collections::HashMap;

use rusqlite::params;

use crate::db::{Tx, UniqueConstraint};
use crate::error::{Error, Fail};
use crate::ledger::FieldValue;
use crate::spec::{Field, FieldName, Link, LinkName, SchemaName, Spec};
use crate::sql::{SqlVal, instant_to_sql, quote_ident, sql_default, sql_type};
use crate::time::Instant;

pub fn insert_schema(tx: &mut Tx<'_>, name: &SchemaName, spec: &Spec) -> Result<(), Error> {
    let yaml = spec.to_yaml()?;
    let cols = create_columns(spec);
    let sql = format!("CREATE TABLE {} ({cols})", quote_ident(name.as_str()));
    tx.as_ref()
        .execute(
            "INSERT INTO schemas (name, spec, retired) VALUES (?1, ?2, 0)",
            params![name.as_str(), yaml],
        )
        .unique(Fail::SchemaExists(name.clone()))?;
    tx.as_ref().execute_batch(&sql)?;
    Ok(())
}

pub fn add_column(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    field: &Field,
    default: Option<&FieldValue>,
) -> Result<(), Error> {
    let table = quote_ident(schema.as_str());
    let col_type = sql_type(field.type_);
    let alter = if let Some(def) = default {
        let sql_def = sql_default(def);
        format!(
            "ALTER TABLE {table} ADD COLUMN {} {col_type} NOT NULL DEFAULT {sql_def}",
            quote_ident(field.name.as_str())
        )
    } else {
        format!(
            "ALTER TABLE {table} ADD COLUMN {} {col_type}",
            quote_ident(field.name.as_str())
        )
    };
    tx.as_ref().execute_batch(&alter)?;
    Ok(())
}

pub fn save_spec(tx: &mut Tx<'_>, name: &SchemaName, spec: &Spec) -> Result<(), Error> {
    let yaml = spec.to_yaml()?;
    tx.as_ref().execute(
        "UPDATE schemas SET spec = ?1 WHERE name = ?2",
        params![yaml, name.as_str()],
    )?;
    Ok(())
}

pub fn retire(tx: &mut Tx<'_>, name: &SchemaName) -> Result<(), Error> {
    let n = tx.as_ref().execute(
        "UPDATE schemas SET retired = 1 WHERE name = ?1",
        [name.as_str()],
    )?;
    if n == 0 {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    }
    Ok(())
}

pub fn drop_schema(tx: &mut Tx<'_>, name: &SchemaName) -> Result<(), Error> {
    let n = tx
        .as_ref()
        .execute("DELETE FROM schemas WHERE name = ?1", [name.as_str()])?;
    if n == 0 {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    }
    tx.as_ref()
        .execute("DELETE FROM links WHERE from_schema = ?1", [name.as_str()])?;
    tx.as_ref()
        .execute_batch(&format!("DROP TABLE {}", quote_ident(name.as_str())))?;
    Ok(())
}

pub fn insert_entry(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    spec: &Spec,
    at: Instant,
    agent: Option<&str>,
    values: &HashMap<FieldName, FieldValue>,
    links: &[Link],
) -> Result<i64, Error> {
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
        quote_ident(schema.as_str()),
        col_names
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", "),
        placeholders.join(", ")
    );
    {
        let mut stmt = tx.as_ref().prepare(&sql)?;
        stmt.execute(rusqlite::params_from_iter(
            bind.iter().map(SqlVal::as_param),
        ))?;
    }
    let id = tx.as_ref().last_insert_rowid();
    insert_links(tx, schema, id, links)?;
    Ok(id)
}

pub fn update_entry(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: i64,
    at: Option<Instant>,
    agent: Option<&str>,
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
    bind.push(SqlVal::Int(id));
    let sql = format!(
        "UPDATE {} SET {} WHERE id = ?{}",
        quote_ident(schema.as_str()),
        sets.join(", "),
        bind.len()
    );
    tx.as_ref().execute(
        &sql,
        rusqlite::params_from_iter(bind.iter().map(SqlVal::as_param)),
    )?;
    Ok(())
}

pub fn delete_link(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: i64,
    name: &LinkName,
) -> Result<(), Error> {
    tx.as_ref().execute(
        "DELETE FROM links WHERE from_schema = ?1 AND from_id = ?2 AND name = ?3",
        params![schema.as_str(), id, name.as_str()],
    )?;
    Ok(())
}

pub fn upsert_link(
    tx: &mut Tx<'_>,
    schema: &SchemaName,
    id: i64,
    link: &Link,
) -> Result<(), Error> {
    tx.as_ref().execute(
        "INSERT INTO links (from_schema, from_id, name, to_schema, to_id)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (from_schema, from_id, name) DO UPDATE SET
            to_schema = excluded.to_schema,
            to_id = excluded.to_id",
        params![
            schema.as_str(),
            id,
            link.name.as_str(),
            link.to.schema.as_str(),
            link.to.id
        ],
    )?;
    Ok(())
}

pub fn set_ignored(tx: &mut Tx<'_>, schema: &SchemaName, id: i64) -> Result<(), Error> {
    tx.as_ref().execute(
        &format!(
            "UPDATE {} SET ignored = 1 WHERE id = ?1",
            quote_ident(schema.as_str())
        ),
        [id],
    )?;
    Ok(())
}

fn insert_links(tx: &Tx<'_>, schema: &SchemaName, id: i64, links: &[Link]) -> Result<(), Error> {
    for link in links {
        tx.as_ref().execute(
            "INSERT INTO links (from_schema, from_id, name, to_schema, to_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                schema.as_str(),
                id,
                link.name.as_str(),
                link.to.schema.as_str(),
                link.to.id
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
        let mut col = format!(
            "{} {}",
            quote_ident(field.name.as_str()),
            sql_type(field.type_)
        );
        if field.required {
            col.push_str(" NOT NULL");
        }
        cols.push(col);
    }
    cols.join(", ")
}
