use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::params;

use crate::error::Error;
use crate::spec::{
    Field, FieldType, Spec, fold_enum, fold_enum_values, is_ident, is_reserved, parse_number,
    parse_target, quote_ident, table_name,
};
use crate::store::{Amend, load_links, load_row, load_schema, require_schema_name, schema_exists};
use crate::time;
use crate::tsv;
use crate::value::{
    SqlVal, field_sql, format_links, format_links_pairs, insert_links, sql_type,
    validate_link_name, write_id_row,
};

pub(crate) struct Tx<'a> {
    inner: rusqlite::Transaction<'a>,
    default_agent: &'a Option<String>,
}

impl<'a> Tx<'a> {
    pub(crate) fn begin(
        conn: &'a mut rusqlite::Connection,
        default_agent: &'a Option<String>,
    ) -> Result<Self, Error> {
        Ok(Self {
            inner: conn.transaction()?,
            default_agent,
        })
    }

    pub(crate) fn commit(self) -> Result<(), Error> {
        self.inner.commit().map_err(Error::from)
    }
}

impl Tx<'_> {
    pub(crate) fn schema_add(&mut self, name: &str, file: &Path) -> Result<String, Error> {
        require_schema_name(name)?;
        if schema_exists(&self.inner, name)? {
            return Err(Error::fail(format!("schema exists: {name}")));
        }
        let raw = std::fs::read_to_string(file)?;
        let spec = Spec::parse_yaml(&raw)?;
        let yaml = spec.to_yaml()?;
        let table = table_name(name);
        let cols = create_columns(&spec);
        let sql = format!("CREATE TABLE {} ({cols})", quote_ident(&table));
        self.inner.execute(
            "INSERT INTO schemas (name, spec, retired) VALUES (?1, ?2, 0)",
            params![name, yaml],
        )?;
        self.inner.execute_batch(&sql)?;
        Ok(String::new())
    }

    pub(crate) fn schema_add_field(
        &mut self,
        schema: &str,
        name: &str,
        type_: FieldType,
        values: Option<Vec<String>>,
        default: Option<String>,
    ) -> Result<String, Error> {
        if !is_ident(name) {
            return Err(Error::fail(format!("invalid field name: {name}")));
        }
        if is_reserved(name) {
            return Err(Error::fail(format!("reserved field name: {name}")));
        }
        let mut row = load_schema(&self.inner, schema)?;
        if row.retired {
            return Err(Error::fail(format!("schema is retired: {schema}")));
        }
        if row.spec.field(name).is_some() {
            return Err(Error::fail(format!("field exists: {name}")));
        }
        let mut values = values;
        if type_ == FieldType::Enum {
            let Some(vals) = values.as_mut() else {
                return Err(Error::usage("--values is required for enum"));
            };
            fold_enum_values(vals)?;
        } else if values.is_some() {
            return Err(Error::usage("--values is only valid for enum"));
        }
        let required = default.is_some();
        if let Some(ref def) = default {
            field_sql(
                &Field {
                    name: name.to_string(),
                    type_,
                    required: true,
                    values: values.clone(),
                },
                def,
            )?;
        }
        let field = Field {
            name: name.to_string(),
            type_,
            required,
            values,
        };
        let table = table_name(schema);
        let col_type = sql_type(type_);
        let alter = if let Some(ref def) = default {
            let sql_def = sql_default(type_, def)?;
            format!(
                "ALTER TABLE {} ADD COLUMN {} {col_type} NOT NULL DEFAULT {sql_def}",
                quote_ident(&table),
                quote_ident(name)
            )
        } else {
            format!(
                "ALTER TABLE {} ADD COLUMN {} {col_type}",
                quote_ident(&table),
                quote_ident(name)
            )
        };
        row.spec.fields.push(field);
        let yaml = row.spec.to_yaml()?;
        self.inner.execute_batch(&alter)?;
        self.inner.execute(
            "UPDATE schemas SET spec = ?1 WHERE name = ?2",
            params![yaml, schema],
        )?;
        Ok(String::new())
    }

    pub(crate) fn schema_add_value(
        &mut self,
        schema: &str,
        field: &str,
        value: &str,
    ) -> Result<String, Error> {
        let mut row = load_schema(&self.inner, schema)?;
        if row.retired {
            return Err(Error::fail(format!("schema is retired: {schema}")));
        }
        let Some(f) = row.spec.fields.iter_mut().find(|f| f.name == field) else {
            return Err(Error::fail(format!("unknown field: {field}")));
        };
        if f.type_ != FieldType::Enum {
            return Err(Error::fail(format!("field is not enum: {field}")));
        }
        let folded = fold_enum(value);
        let values = f.values.get_or_insert_with(Vec::new);
        if values.iter().any(|v| v == &folded) {
            return Err(Error::fail(format!("enum value exists: {folded}")));
        }
        values.push(folded);
        let yaml = row.spec.to_yaml()?;
        self.inner.execute(
            "UPDATE schemas SET spec = ?1 WHERE name = ?2",
            params![yaml, schema],
        )?;
        Ok(String::new())
    }

    pub(crate) fn schema_retire(&mut self, name: &str) -> Result<String, Error> {
        let n = self
            .inner
            .execute("UPDATE schemas SET retired = 1 WHERE name = ?1", [name])?;
        if n == 0 {
            return Err(Error::fail(format!("unknown schema: {name}")));
        }
        Ok(String::new())
    }

    pub(crate) fn schema_drop(&mut self, name: &str) -> Result<String, Error> {
        if !schema_exists(&self.inner, name)? {
            return Err(Error::fail(format!("unknown schema: {name}")));
        }
        let inbound: i64 = self.inner.query_row(
            "SELECT COUNT(*) FROM links WHERE to_schema = ?1",
            [name],
            |row| row.get(0),
        )?;
        if inbound > 0 {
            return Err(Error::fail(format!(
                "schema {name} still has inbound links"
            )));
        }
        let table = table_name(name);
        self.inner
            .execute("DELETE FROM links WHERE from_schema = ?1", [name])?;
        self.inner
            .execute_batch(&format!("DROP TABLE {}", quote_ident(&table)))?;
        self.inner
            .execute("DELETE FROM schemas WHERE name = ?1", [name])?;
        Ok(String::new())
    }

    pub(crate) fn log(
        &mut self,
        schema: &str,
        at: Option<&str>,
        agent: Option<&str>,
        links: &[(String, String)],
        fields: &[(String, String)],
    ) -> Result<String, Error> {
        let row = load_schema(&self.inner, schema)?;
        if row.retired {
            return Err(Error::fail(format!("schema is retired: {schema}")));
        }
        let at = match at {
            Some(s) => time::parse_instant(s)?,
            None => time::now_stored()?,
        };
        let agent = agent
            .map(str::to_string)
            .or_else(|| self.default_agent.clone());
        let values = prepare_fields(&row.spec, fields, false)?;
        let prepared_links = self.prepare_links(&row.spec, links)?;
        let table = table_name(schema);
        let mut col_names = vec!["at".to_string(), "agent".to_string()];
        let mut placeholders = vec!["?1".to_string(), "?2".to_string()];
        let mut bind: Vec<SqlVal> = vec![
            SqlVal::Text(at.clone()),
            match &agent {
                Some(a) => SqlVal::Text(a.clone()),
                None => SqlVal::Null,
            },
        ];
        for field in &row.spec.fields {
            col_names.push(field.name.clone());
            placeholders.push(format!("?{}", bind.len() + 1));
            bind.push(values.get(&field.name).cloned().unwrap_or(SqlVal::Null));
        }
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_ident(&table),
            col_names
                .iter()
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", "),
            placeholders.join(", ")
        );
        {
            let mut stmt = self.inner.prepare(&sql)?;
            stmt.execute(rusqlite::params_from_iter(
                bind.iter().map(SqlVal::as_param),
            ))?;
        }
        let id = self.inner.last_insert_rowid();
        insert_links(&self.inner, schema, id, &prepared_links)?;
        write_id_row(id, &at, &format_links(&prepared_links))
    }

    pub(crate) fn amend(
        &mut self,
        schema: &str,
        id: i64,
        change: Amend<'_>,
    ) -> Result<String, Error> {
        if change.at.is_none()
            && change.agent.is_none()
            && change.links.is_empty()
            && change.unlinks.is_empty()
            && change.fields.is_empty()
        {
            return Err(Error::usage("amend requires at least one change"));
        }
        let row = load_schema(&self.inner, schema)?;
        if load_row(&self.inner, schema, &row.spec, id)?.is_none() {
            return Err(Error::fail(format!("not found: {schema}/{id}")));
        }
        let mut unlink_set = HashSet::new();
        for name in change.unlinks {
            if !unlink_set.insert(name.as_str()) {
                return Err(Error::usage(format!("duplicate --unlink {name}")));
            }
            if change.links.iter().any(|(n, _)| n == name) {
                return Err(Error::usage(format!(
                    "--link and --unlink of the same name: {name}"
                )));
            }
        }
        let values = prepare_fields(&row.spec, change.fields, true)?;
        let prepared_links = self.prepare_links(&row.spec, change.links)?;
        let table = table_name(schema);
        let mut sets = Vec::new();
        let mut bind: Vec<SqlVal> = Vec::new();
        if let Some(at) = change.at {
            let stored = time::parse_instant(at)?;
            sets.push(format!("at = ?{}", bind.len() + 1));
            bind.push(SqlVal::Text(stored));
        }
        if let Some(agent) = change.agent {
            sets.push(format!("agent = ?{}", bind.len() + 1));
            bind.push(SqlVal::Text(agent.to_string()));
        }
        for (name, val) in &values {
            sets.push(format!("{} = ?{}", quote_ident(name), bind.len() + 1));
            bind.push(val.clone());
        }
        if !sets.is_empty() {
            bind.push(SqlVal::Int(id));
            let sql = format!(
                "UPDATE {} SET {} WHERE id = ?{}",
                quote_ident(&table),
                sets.join(", "),
                bind.len()
            );
            self.inner.execute(
                &sql,
                rusqlite::params_from_iter(bind.iter().map(SqlVal::as_param)),
            )?;
        }
        for name in change.unlinks {
            self.inner.execute(
                "DELETE FROM links WHERE from_schema = ?1 AND from_id = ?2 AND name = ?3",
                params![schema, id, name],
            )?;
        }
        for (name, to_schema, to_id) in &prepared_links {
            self.inner.execute(
                "INSERT INTO links (from_schema, from_id, name, to_schema, to_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (from_schema, from_id, name) DO UPDATE SET
                    to_schema = excluded.to_schema,
                    to_id = excluded.to_id",
                params![schema, id, name, to_schema, to_id],
            )?;
        }
        let at_now: String = self.inner.query_row(
            &format!("SELECT at FROM {} WHERE id = ?1", quote_ident(&table)),
            [id],
            |r| r.get(0),
        )?;
        let links_cell = format_links_pairs(&load_links(&self.inner, schema, id)?);
        write_id_row(id, &at_now, &links_cell)
    }

    pub(crate) fn ignore(&mut self, schema: &str, id: i64) -> Result<String, Error> {
        let spec = load_schema(&self.inner, schema)?.spec;
        let Some(row) = load_row(&self.inner, schema, &spec, id)? else {
            return Err(Error::fail(format!("not found: {schema}/{id}")));
        };
        let table = table_name(schema);
        self.inner.execute(
            &format!(
                "UPDATE {} SET ignored = 1 WHERE id = ?1",
                quote_ident(&table)
            ),
            [id],
        )?;
        let at = time::display_local(&row.at)?;
        Ok(tsv::table(&["id", "at"], &[vec![id.to_string(), at]]))
    }

    fn prepare_links(
        &self,
        spec: &Spec,
        links: &[(String, String)],
    ) -> Result<Vec<(String, String, i64)>, Error> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for (name, target) in links {
            if !seen.insert(name.as_str()) {
                return Err(Error::usage(format!("link name once per command: {name}")));
            }
            validate_link_name(name, spec)?;
            let (to_schema, to_id) = parse_target(target)?;
            let target_spec = load_schema(&self.inner, &to_schema)?.spec;
            if load_row(&self.inner, &to_schema, &target_spec, to_id)?.is_none() {
                return Err(Error::fail(format!(
                    "link target missing: {to_schema}/{to_id}"
                )));
            }
            out.push((name.clone(), to_schema, to_id));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }
}

fn prepare_fields(
    spec: &Spec,
    fields: &[(String, String)],
    partial: bool,
) -> Result<HashMap<String, SqlVal>, Error> {
    let mut seen = HashSet::new();
    let mut out = HashMap::new();
    for (name, value) in fields {
        if !seen.insert(name.as_str()) {
            return Err(Error::usage(format!("duplicate field: {name}")));
        }
        let Some(field) = spec.field(name) else {
            return Err(Error::fail(format!("unknown field: {name}")));
        };
        if value.is_empty() {
            if field.required {
                return Err(Error::fail(format!("missing required field: {name}")));
            }
            out.insert(name.clone(), SqlVal::Null);
            continue;
        }
        out.insert(name.clone(), field_sql(field, value)?);
    }
    if !partial {
        for field in &spec.fields {
            if field.required && !out.contains_key(&field.name) {
                return Err(Error::fail(format!(
                    "missing required field: {}",
                    field.name
                )));
            }
        }
    }
    Ok(out)
}

fn sql_default(t: FieldType, def: &str) -> Result<String, Error> {
    match t {
        FieldType::Number => {
            parse_number(def)?;
            Ok(def.to_string())
        }
        FieldType::Text | FieldType::Enum => Ok(format!("'{}'", def.replace('\'', "''"))),
    }
}

fn create_columns(spec: &Spec) -> String {
    let mut cols = vec![
        "id INTEGER PRIMARY KEY".to_string(),
        "at TEXT NOT NULL".to_string(),
        "agent TEXT".to_string(),
        "ignored INTEGER NOT NULL DEFAULT 0".to_string(),
    ];
    for field in &spec.fields {
        let mut col = format!("{} {}", quote_ident(&field.name), sql_type(field.type_));
        if field.required {
            col.push_str(" NOT NULL");
        }
        cols.push(col);
    }
    cols.join(", ")
}
