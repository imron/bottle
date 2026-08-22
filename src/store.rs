use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::cmd::Cmd;
use crate::error::Error;
use crate::spec::{
    FieldType, Spec, fold_enum, is_ident, is_reserved, is_schema_name, is_time_group, parse_number,
    parse_target, quote_ident, table_name,
};
use crate::time;
use crate::tsv;
use crate::value::{SqlVal, format_links_pairs};

pub struct Bottle {
    conn: Connection,
    default_agent: Option<String>,
}

pub(crate) struct SchemaRow {
    pub spec: Spec,
    pub retired: bool,
}

pub(crate) struct Filter<'a> {
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub agent: Option<&'a str>,
    pub wheres: &'a [(String, String)],
}

pub(crate) struct Amend<'a> {
    pub at: Option<&'a str>,
    pub agent: Option<&'a str>,
    pub links: &'a [(String, String)],
    pub unlinks: &'a [String],
    pub fields: &'a [(String, String)],
}

pub(crate) struct DataRow {
    pub id: i64,
    pub at: String,
    pub agent: Option<String>,
    pub ignored: bool,
    pub cells: HashMap<String, String>,
    pub numbers: HashMap<String, f64>,
    pub links: Vec<(String, String)>,
}

pub(crate) enum QueryOrder {
    AtIdAsc,
    AtIdDesc,
}

impl Bottle {
    pub fn open(path: &Path, default_agent: Option<String>) -> Result<Self, Error> {
        Ok(Self {
            conn: crate::db::open(path)?,
            default_agent,
        })
    }

    pub(crate) fn transaction<T>(
        &mut self,
        f: impl FnOnce(&mut crate::mutable_store::Tx<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut tx = crate::mutable_store::Tx::begin(&mut self.conn, &self.default_agent)?;
        match f(&mut tx) {
            Ok(value) => {
                tx.commit()?;
                Ok(value)
            }
            Err(err) => Err(err),
        }
    }

    pub fn run(&mut self, cmd: Cmd) -> Result<String, Error> {
        match cmd {
            Cmd::Help { topic } => crate::help::page(topic.as_deref()),
            Cmd::SchemaList => self.schema_list(),
            Cmd::SchemaShow { name, yaml } => self.schema_show(&name, yaml),
            Cmd::SchemaAdd { name, file } => self.transaction(|tx| tx.schema_add(&name, &file)),
            Cmd::SchemaAddField {
                schema,
                name,
                type_,
                values,
                default,
            } => self.transaction(|tx| tx.schema_add_field(&schema, &name, type_, values, default)),
            Cmd::SchemaAddValue {
                schema,
                field,
                value,
            } => self.transaction(|tx| tx.schema_add_value(&schema, &field, &value)),
            Cmd::SchemaRetire { name } => self.transaction(|tx| tx.schema_retire(&name)),
            Cmd::SchemaDrop { name } => self.transaction(|tx| tx.schema_drop(&name)),
            Cmd::Log {
                schema,
                at,
                agent,
                links,
                fields,
            } => self.transaction(|tx| {
                tx.log(&schema, at.as_deref(), agent.as_deref(), &links, &fields)
            }),
            Cmd::Ls {
                schema,
                from,
                to,
                agent,
                wheres,
                include_ignored,
            } => self.ls(
                &schema,
                Filter {
                    from: from.as_deref(),
                    to: to.as_deref(),
                    agent: agent.as_deref(),
                    wheres: &wheres,
                },
                include_ignored,
            ),
            Cmd::Get { schema, id } => self.get(&schema, id),
            Cmd::Sum {
                schema,
                field,
                from,
                to,
                agent,
                wheres,
                group,
            } => self.sum(
                &schema,
                &field,
                Filter {
                    from: from.as_deref(),
                    to: to.as_deref(),
                    agent: agent.as_deref(),
                    wheres: &wheres,
                },
                group.as_deref(),
            ),
            Cmd::Last {
                schema,
                agent,
                wheres,
            } => self.last(&schema, agent.as_deref(), &wheres),
            Cmd::Today {
                schema,
                agent,
                wheres,
            } => self.today(&schema, agent.as_deref(), &wheres),
            Cmd::Amend {
                schema,
                id,
                at,
                agent,
                links,
                unlinks,
                fields,
            } => self.transaction(|tx| {
                tx.amend(
                    &schema,
                    id,
                    Amend {
                        at: at.as_deref(),
                        agent: agent.as_deref(),
                        links: &links,
                        unlinks: &unlinks,
                        fields: &fields,
                    },
                )
            }),
            Cmd::Ignore { schema, id } => self.transaction(|tx| tx.ignore(&schema, id)),
        }
    }

    fn schema_list(&self) -> Result<String, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, retired FROM schemas ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let retired: i64 = row.get(1)?;
            Ok(vec![name, tsv::bool_cell(retired != 0).to_string()])
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(tsv::table(&["name", "retired"], &out))
    }

    fn schema_show(&self, name: &str, yaml: bool) -> Result<String, Error> {
        let schema = load_schema(&self.conn, name)?;
        if yaml {
            return schema.spec.to_yaml();
        }
        let mut rows = Vec::new();
        for field in &schema.spec.fields {
            let values = match &field.values {
                Some(v) => v.join(","),
                None => String::new(),
            };
            rows.push(vec![
                field.name.clone(),
                type_name(field.type_).to_string(),
                tsv::bool_cell(field.required).to_string(),
                values,
            ]);
        }
        Ok(tsv::table(&["name", "type", "required", "values"], &rows))
    }

    fn ls(&self, schema: &str, filter: Filter<'_>, include_ignored: bool) -> Result<String, Error> {
        let rows = query_rows(
            &self.conn,
            schema,
            filter,
            include_ignored,
            QueryOrder::AtIdAsc,
            None,
        )?;
        let spec = &load_schema(&self.conn, schema)?.spec;
        Ok(format_row_table(spec, &rows, include_ignored))
    }

    fn get(&self, schema: &str, id: i64) -> Result<String, Error> {
        let spec = load_schema(&self.conn, schema)?.spec;
        let Some(row) = load_row(&self.conn, schema, &spec, id)? else {
            return Err(Error::fail(format!("not found: {schema}/{id}")));
        };
        Ok(format_row_table(&spec, &[row], true))
    }

    fn last(
        &self,
        schema: &str,
        agent: Option<&str>,
        wheres: &[(String, String)],
    ) -> Result<String, Error> {
        let rows = query_rows(
            &self.conn,
            schema,
            Filter {
                from: None,
                to: None,
                agent,
                wheres,
            },
            false,
            QueryOrder::AtIdDesc,
            Some(1),
        )?;
        if rows.is_empty() {
            return Err(Error::fail("not found"));
        }
        let spec = &load_schema(&self.conn, schema)?.spec;
        Ok(format_row_table(spec, &rows, false))
    }

    fn today(
        &self,
        schema: &str,
        agent: Option<&str>,
        wheres: &[(String, String)],
    ) -> Result<String, Error> {
        let (start, end) = time::today_window()?;
        self.ls(
            schema,
            Filter {
                from: Some(&start),
                to: Some(&end),
                agent,
                wheres,
            },
            false,
        )
    }

    fn sum(
        &self,
        schema: &str,
        field: &str,
        filter: Filter<'_>,
        group: Option<&str>,
    ) -> Result<String, Error> {
        let spec = load_schema(&self.conn, schema)?.spec;
        let Some(f) = spec.field(field) else {
            return Err(Error::fail(format!("unknown field: {field}")));
        };
        if f.type_ != FieldType::Number {
            return Err(Error::fail(format!("field is not a number: {field}")));
        }
        let rows = query_rows(&self.conn, schema, filter, false, QueryOrder::AtIdAsc, None)?;
        match group {
            None => {
                let total: f64 = rows
                    .iter()
                    .filter_map(|r| r.numbers.get(field).copied())
                    .sum();
                Ok(tsv::table(
                    &["field", "value"],
                    &[vec![field.to_string(), tsv::number(total)]],
                ))
            }
            Some(g) if is_time_group(g) => {
                let mut buckets: BTreeMap<String, f64> = BTreeMap::new();
                for row in &rows {
                    let key = time_group_key(g, &row.at)?;
                    *buckets.entry(key).or_insert(0.0) +=
                        row.numbers.get(field).copied().unwrap_or(0.0);
                }
                if rows.is_empty() {
                    return Ok(tsv::table(&[g, "value"], &[]));
                }
                let out: Vec<Vec<String>> = buckets
                    .into_iter()
                    .map(|(k, v)| vec![k, tsv::number(v)])
                    .collect();
                Ok(tsv::table(&[g, "value"], &out))
            }
            Some(g) => {
                if is_reserved(g) || spec.field(g).is_some() {
                    return Err(Error::fail(format!("invalid group: {g}")));
                }
                let mut buckets: BTreeMap<String, f64> = BTreeMap::new();
                for row in &rows {
                    let key = row
                        .links
                        .iter()
                        .find(|(n, _)| n == g)
                        .map(|(_, t)| t.clone())
                        .unwrap_or_default();
                    *buckets.entry(key).or_insert(0.0) +=
                        row.numbers.get(field).copied().unwrap_or(0.0);
                }
                if rows.is_empty() {
                    return Ok(tsv::table(&[g, "value"], &[]));
                }
                let out: Vec<Vec<String>> = buckets
                    .into_iter()
                    .map(|(k, v)| vec![k, tsv::number(v)])
                    .collect();
                Ok(tsv::table(&[g, "value"], &out))
            }
        }
    }
}

pub(crate) fn load_schema(conn: &Connection, name: &str) -> Result<SchemaRow, Error> {
    require_schema_name(name)?;
    let row = conn
        .query_row(
            "SELECT spec, retired FROM schemas WHERE name = ?1",
            [name],
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
    Ok(SchemaRow {
        spec: Spec::parse_yaml(&spec)?,
        retired: retired != 0,
    })
}

pub(crate) fn schema_exists(conn: &Connection, name: &str) -> Result<bool, Error> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM schemas WHERE name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(n > 0)
}

pub(crate) fn require_schema_name(name: &str) -> Result<(), Error> {
    if is_schema_name(name) {
        Ok(())
    } else {
        Err(Error::fail(format!(
            "schema name must be family.kind: {name}"
        )))
    }
}

pub(crate) fn type_name(t: FieldType) -> &'static str {
    match t {
        FieldType::Text => "text",
        FieldType::Number => "number",
        FieldType::Enum => "enum",
    }
}

pub(crate) fn load_row(
    conn: &Connection,
    schema: &str,
    spec: &Spec,
    id: i64,
) -> Result<Option<DataRow>, Error> {
    let table = table_name(schema);
    let sql = format!("SELECT * FROM {} WHERE id = ?1", quote_ident(&table));
    let mut stmt = conn.prepare(&sql)?;
    let col_count = stmt.column_count();
    let names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();
    let mut raw = stmt.query([id])?;
    match raw.next()? {
        Some(r) => Ok(Some(read_data_row(conn, schema, spec, &names, r)?)),
        None => Ok(None),
    }
}

pub(crate) fn query_rows(
    conn: &Connection,
    schema: &str,
    filter: Filter<'_>,
    include_ignored: bool,
    order: QueryOrder,
    limit: Option<usize>,
) -> Result<Vec<DataRow>, Error> {
    let spec = load_schema(conn, schema)?.spec;
    let table = table_name(schema);
    let mut sql = format!("SELECT * FROM {} WHERE 1=1", quote_ident(&table));
    let mut bind: Vec<SqlVal> = Vec::new();
    if !include_ignored {
        sql.push_str(" AND ignored = 0");
    }
    if let Some(from) = filter.from {
        let start = if looks_stored(from) {
            from.to_string()
        } else {
            time::from_bound(from)?
        };
        bind.push(SqlVal::Text(start));
        sql.push_str(&format!(" AND at >= ?{}", bind.len()));
    }
    if let Some(to) = filter.to {
        if looks_stored(to) && to.ends_with('Z') && to.contains('T') && to.len() == 20 {
            bind.push(SqlVal::Text(to.to_string()));
            sql.push_str(&format!(" AND at < ?{}", bind.len()));
        } else {
            let (end, inclusive) = time::to_bound_sql(to)?;
            bind.push(SqlVal::Text(end));
            if inclusive {
                sql.push_str(&format!(" AND at <= ?{}", bind.len()));
            } else {
                sql.push_str(&format!(" AND at < ?{}", bind.len()));
            }
        }
    }
    if let Some(agent) = filter.agent {
        bind.push(SqlVal::Text(agent.to_string()));
        sql.push_str(&format!(" AND agent = ?{}", bind.len()));
    }
    for (name, value) in filter.wheres {
        if is_reserved(name) {
            return Err(Error::usage(format!(
                "--where {name}= is reserved; use --agent, get, or --from/--to"
            )));
        }
        if let Some(field) = spec.field(name) {
            match field.type_ {
                FieldType::Number => bind.push(SqlVal::Real(parse_number(value)?)),
                FieldType::Enum => bind.push(SqlVal::Text(fold_enum(value))),
                FieldType::Text => bind.push(SqlVal::Text(value.clone())),
            }
            sql.push_str(&format!(" AND {} = ?{}", quote_ident(name), bind.len()));
        } else {
            let (to_schema, to_id) = parse_target(value)?;
            if !is_ident(name) || is_time_group(name) {
                return Err(Error::fail(format!("invalid link name: {name}")));
            }
            bind.push(SqlVal::Text(schema.to_string()));
            bind.push(SqlVal::Text(name.clone()));
            bind.push(SqlVal::Text(to_schema));
            bind.push(SqlVal::Int(to_id));
            let a = bind.len() - 3;
            sql.push_str(&format!(
                " AND id IN (SELECT from_id FROM links WHERE from_schema = ?{a} AND name = ?{} AND to_schema = ?{} AND to_id = ?{})",
                a + 1,
                a + 2,
                a + 3
            ));
        }
    }
    match order {
        QueryOrder::AtIdAsc => sql.push_str(" ORDER BY at ASC, id ASC"),
        QueryOrder::AtIdDesc => sql.push_str(" ORDER BY at DESC, id DESC"),
    }
    if let Some(limit) = limit {
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
    let mut rows = Vec::new();
    while let Some(r) = raw.next()? {
        rows.push(read_data_row(conn, schema, &spec, &names, r)?);
    }
    Ok(rows)
}

fn read_data_row(
    conn: &Connection,
    schema: &str,
    spec: &Spec,
    names: &[String],
    r: &rusqlite::Row<'_>,
) -> Result<DataRow, Error> {
    let mut cells = HashMap::new();
    let mut numbers = HashMap::new();
    let mut id = 0_i64;
    let mut at = String::new();
    let mut agent = None;
    let mut ignored = false;
    for (i, name) in names.iter().enumerate() {
        match name.as_str() {
            "id" => id = r.get(i)?,
            "at" => at = r.get(i)?,
            "agent" => agent = r.get(i)?,
            "ignored" => {
                let v: i64 = r.get(i)?;
                ignored = v != 0;
            }
            other => {
                if let Some(field) = spec.field(other) {
                    match field.type_ {
                        FieldType::Number => {
                            let v: Option<f64> = r.get(i)?;
                            if let Some(n) = v {
                                numbers.insert(other.to_string(), n);
                                cells.insert(other.to_string(), tsv::number(n));
                            } else {
                                cells.insert(other.to_string(), String::new());
                            }
                        }
                        _ => {
                            let v: Option<String> = r.get(i)?;
                            cells.insert(other.to_string(), v.unwrap_or_default());
                        }
                    }
                }
            }
        }
    }
    Ok(DataRow {
        id,
        at,
        agent,
        ignored,
        cells,
        numbers,
        links: load_links(conn, schema, id)?,
    })
}

pub(crate) fn load_links(
    conn: &Connection,
    schema: &str,
    id: i64,
) -> Result<Vec<(String, String)>, Error> {
    let mut stmt = conn.prepare(
        "SELECT name, to_schema, to_id FROM links
         WHERE from_schema = ?1 AND from_id = ?2
         ORDER BY name",
    )?;
    let rows = stmt.query_map(params![schema, id], |row| {
        let name: String = row.get(0)?;
        let to_schema: String = row.get(1)?;
        let to_id: i64 = row.get(2)?;
        Ok((name, format!("{to_schema}/{to_id}")))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn format_row_table(spec: &Spec, rows: &[DataRow], show_ignored: bool) -> String {
    let mut headers = vec!["id".to_string(), "at".to_string(), "links".to_string()];
    for field in &spec.fields {
        headers.push(field.name.clone());
    }
    headers.push("agent".to_string());
    if show_ignored {
        headers.push("ignored".to_string());
    }
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let mut out_rows = Vec::new();
    for row in rows {
        let mut cells = vec![
            row.id.to_string(),
            time::display_local(&row.at).unwrap_or_default(),
            format_links_pairs(&row.links),
        ];
        for field in &spec.fields {
            cells.push(row.cells.get(&field.name).cloned().unwrap_or_default());
        }
        cells.push(row.agent.clone().unwrap_or_default());
        if show_ignored {
            cells.push(tsv::bool_cell(row.ignored).to_string());
        }
        out_rows.push(cells);
    }
    tsv::table(&header_refs, &out_rows)
}

fn time_group_key(group: &str, stored: &str) -> Result<String, Error> {
    let date = time::local_civil(stored)?;
    match group {
        "day" => Ok(date.to_string()),
        "month" => Ok(format!("{:04}-{:02}", date.year(), date.month())),
        "year" => Ok(format!("{:04}", date.year())),
        "week" => {
            let iso = date.iso_week_date();
            Ok(format!("{}-W{:02}", iso.year(), iso.week()))
        }
        _ => Err(Error::fail(format!("invalid group: {group}"))),
    }
}

fn looks_stored(s: &str) -> bool {
    s.len() == 20 && s.ends_with('Z') && s.contains('T')
}
