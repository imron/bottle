use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Error;
use crate::spec::{
    FieldType, Spec, fold_enum, is_ident, is_reserved, is_schema_name, is_time_group, parse_number,
    parse_target, quote_ident, table_name,
};
use crate::time::{Range, ToBound};
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

pub(crate) struct Amend<'a> {
    pub at: Option<&'a str>,
    pub agent: Option<&'a str>,
    pub links: &'a [(String, String)],
    pub unlinks: &'a [String],
    pub fields: &'a [(String, String)],
}

enum Cell {
    Empty,
    Text(String),
    Number(f64),
}

impl Cell {
    fn tsv(&self) -> String {
        match self {
            Cell::Empty => String::new(),
            Cell::Text(s) => s.clone(),
            Cell::Number(n) => tsv::number(*n),
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Cell::Number(n) => Some(*n),
            _ => None,
        }
    }
}

pub(crate) struct DataRow {
    pub id: i64,
    pub at: String,
    pub agent: Option<String>,
    pub ignored: bool,
    cells: HashMap<String, Cell>,
    pub links: Vec<(String, String)>,
}

#[derive(Clone, Copy)]
enum QueryOrder {
    AtIdAsc,
    AtIdDesc,
}

enum WhereAtom {
    Field {
        name: String,
        value: SqlVal,
    },
    Link {
        name: String,
        to_schema: String,
        to_id: i64,
    },
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

    pub fn schema_list(&self) -> Result<String, Error> {
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

    pub fn schema_show(&self, name: &str, yaml: bool) -> Result<String, Error> {
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

    pub fn ls(
        &self,
        schema: &str,
        range: Range,
        agent: Option<&str>,
        wheres: &[(String, String)],
        include_ignored: bool,
    ) -> Result<String, Error> {
        let (spec, rows) = fetch(
            &self.conn,
            Fetch {
                schema,
                range,
                agent,
                wheres,
                include_ignored,
                order: QueryOrder::AtIdAsc,
                limit: None,
            },
        )?;
        format_row_table(&spec, &rows, include_ignored)
    }

    pub fn get(&self, schema: &str, id: i64) -> Result<String, Error> {
        let spec = load_schema(&self.conn, schema)?.spec;
        let Some(row) = load_row(&self.conn, schema, &spec, id)? else {
            return Err(Error::fail(format!("not found: {schema}/{id}")));
        };
        format_row_table(&spec, std::slice::from_ref(&row), true)
    }

    pub fn last(
        &self,
        schema: &str,
        agent: Option<&str>,
        wheres: &[(String, String)],
    ) -> Result<String, Error> {
        let (spec, rows) = fetch(
            &self.conn,
            Fetch {
                schema,
                range: Range::default(),
                agent,
                wheres,
                include_ignored: false,
                order: QueryOrder::AtIdDesc,
                limit: Some(1),
            },
        )?;
        if rows.is_empty() {
            return Err(Error::fail("not found"));
        }
        format_row_table(&spec, &rows, false)
    }

    pub fn today(
        &self,
        schema: &str,
        agent: Option<&str>,
        wheres: &[(String, String)],
    ) -> Result<String, Error> {
        self.ls(schema, Range::today()?, agent, wheres, false)
    }

    pub fn sum(
        &self,
        schema: &str,
        field: &str,
        range: Range,
        agent: Option<&str>,
        wheres: &[(String, String)],
        group: Option<&str>,
    ) -> Result<String, Error> {
        let (spec, rows) = fetch(
            &self.conn,
            Fetch {
                schema,
                range,
                agent,
                wheres,
                include_ignored: false,
                order: QueryOrder::AtIdAsc,
                limit: None,
            },
        )?;
        let Some(f) = spec.field(field) else {
            return Err(Error::fail(format!("unknown field: {field}")));
        };
        if f.type_ != FieldType::Number {
            return Err(Error::fail(format!("field is not a number: {field}")));
        }
        match group {
            None => {
                let total: f64 = rows.iter().filter_map(|r| r.number(field)).sum();
                Ok(tsv::table(
                    &["field", "value"],
                    &[vec![field.to_string(), tsv::number(total)]],
                ))
            }
            Some(g) if is_time_group(g) => {
                let buckets = group_sum(&rows, field, |row| time_group_key(g, &row.at))?;
                group_table(g, &rows, buckets)
            }
            Some(g) => {
                if is_reserved(g) || spec.field(g).is_some() {
                    return Err(Error::fail(format!("invalid group: {g}")));
                }
                let buckets = group_sum(&rows, field, |row| {
                    Ok(row
                        .links
                        .iter()
                        .find(|(n, _)| n == g)
                        .map(|(_, t)| t.clone())
                        .unwrap_or_default())
                })?;
                group_table(g, &rows, buckets)
            }
        }
    }
}

impl DataRow {
    fn number(&self, field: &str) -> Option<f64> {
        self.cells.get(field).and_then(Cell::number)
    }
}

struct Fetch<'a> {
    schema: &'a str,
    range: Range,
    agent: Option<&'a str>,
    wheres: &'a [(String, String)],
    include_ignored: bool,
    order: QueryOrder,
    limit: Option<usize>,
}

fn fetch(conn: &Connection, q: Fetch<'_>) -> Result<(Spec, Vec<DataRow>), Error> {
    let spec = load_schema(conn, q.schema)?.spec;
    let atoms = resolve_wheres(&spec, q.wheres)?;
    let rows = execute_select(conn, &spec, &q, &atoms)?;
    Ok((spec, rows))
}

fn resolve_wheres(spec: &Spec, wheres: &[(String, String)]) -> Result<Vec<WhereAtom>, Error> {
    let mut atoms = Vec::new();
    for (name, value) in wheres {
        if is_reserved(name) {
            return Err(Error::usage(format!(
                "--where {name}= is reserved; use --agent, get, or --from/--to"
            )));
        }
        if let Some(field) = spec.field(name) {
            let value = match field.type_ {
                FieldType::Number => SqlVal::Real(parse_number(value)?),
                FieldType::Enum => SqlVal::Text(fold_enum(value)),
                FieldType::Text => SqlVal::Text(value.clone()),
            };
            atoms.push(WhereAtom::Field {
                name: name.clone(),
                value,
            });
        } else {
            if !is_ident(name) || is_time_group(name) {
                return Err(Error::fail(format!("invalid link name: {name}")));
            }
            let (to_schema, to_id) = parse_target(value)?;
            atoms.push(WhereAtom::Link {
                name: name.clone(),
                to_schema,
                to_id,
            });
        }
    }
    Ok(atoms)
}

fn execute_select(
    conn: &Connection,
    spec: &Spec,
    q: &Fetch<'_>,
    atoms: &[WhereAtom],
) -> Result<Vec<DataRow>, Error> {
    let table = table_name(q.schema);
    let mut sql = format!("SELECT * FROM {} WHERE 1=1", quote_ident(&table));
    let mut bind: Vec<SqlVal> = Vec::new();
    if !q.include_ignored {
        sql.push_str(" AND ignored = 0");
    }
    if let Some(from) = &q.range.from {
        bind.push(SqlVal::Text(from.clone()));
        sql.push_str(&format!(" AND at >= ?{}", bind.len()));
    }
    if let Some(to) = &q.range.to {
        match to {
            ToBound::Inclusive(end) => {
                bind.push(SqlVal::Text(end.clone()));
                sql.push_str(&format!(" AND at <= ?{}", bind.len()));
            }
            ToBound::Exclusive(end) => {
                bind.push(SqlVal::Text(end.clone()));
                sql.push_str(&format!(" AND at < ?{}", bind.len()));
            }
        }
    }
    if let Some(agent) = q.agent {
        bind.push(SqlVal::Text(agent.to_string()));
        sql.push_str(&format!(" AND agent = ?{}", bind.len()));
    }
    for atom in atoms {
        match atom {
            WhereAtom::Field { name, value } => {
                bind.push(value.clone());
                sql.push_str(&format!(" AND {} = ?{}", quote_ident(name), bind.len()));
            }
            WhereAtom::Link {
                name,
                to_schema,
                to_id,
            } => {
                bind.push(SqlVal::Text(q.schema.to_string()));
                bind.push(SqlVal::Text(name.clone()));
                bind.push(SqlVal::Text(to_schema.clone()));
                bind.push(SqlVal::Int(*to_id));
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
        QueryOrder::AtIdAsc => sql.push_str(" ORDER BY at ASC, id ASC"),
        QueryOrder::AtIdDesc => sql.push_str(" ORDER BY at DESC, id DESC"),
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
    let mut rows = Vec::new();
    while let Some(r) = raw.next()? {
        rows.push(read_data_row(conn, q.schema, spec, &names, r)?);
    }
    Ok(rows)
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

fn type_name(t: FieldType) -> &'static str {
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

fn read_data_row(
    conn: &Connection,
    schema: &str,
    spec: &Spec,
    names: &[String],
    r: &rusqlite::Row<'_>,
) -> Result<DataRow, Error> {
    let mut cells = HashMap::new();
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
                            cells.insert(
                                other.to_string(),
                                match v {
                                    Some(n) => Cell::Number(n),
                                    None => Cell::Empty,
                                },
                            );
                        }
                        _ => {
                            let v: Option<String> = r.get(i)?;
                            cells.insert(
                                other.to_string(),
                                match v {
                                    Some(s) if !s.is_empty() => Cell::Text(s),
                                    _ => Cell::Empty,
                                },
                            );
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

fn format_row_table(spec: &Spec, rows: &[DataRow], show_ignored: bool) -> Result<String, Error> {
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
            crate::time::display_local(&row.at)?,
            format_links_pairs(&row.links),
        ];
        for field in &spec.fields {
            cells.push(
                row.cells
                    .get(&field.name)
                    .map(Cell::tsv)
                    .unwrap_or_default(),
            );
        }
        cells.push(row.agent.clone().unwrap_or_default());
        if show_ignored {
            cells.push(tsv::bool_cell(row.ignored).to_string());
        }
        out_rows.push(cells);
    }
    Ok(tsv::table(&header_refs, &out_rows))
}

fn group_sum(
    rows: &[DataRow],
    field: &str,
    key: impl Fn(&DataRow) -> Result<String, Error>,
) -> Result<BTreeMap<String, f64>, Error> {
    let mut buckets: BTreeMap<String, f64> = BTreeMap::new();
    for row in rows {
        let k = key(row)?;
        *buckets.entry(k).or_insert(0.0) += row.number(field).unwrap_or(0.0);
    }
    Ok(buckets)
}

fn group_table(
    name: &str,
    rows: &[DataRow],
    buckets: BTreeMap<String, f64>,
) -> Result<String, Error> {
    if rows.is_empty() {
        return Ok(tsv::table(&[name, "value"], &[]));
    }
    let out: Vec<Vec<String>> = buckets
        .into_iter()
        .map(|(k, v)| vec![k, tsv::number(v)])
        .collect();
    Ok(tsv::table(&[name, "value"], &out))
}

fn time_group_key(group: &str, stored: &str) -> Result<String, Error> {
    let date = crate::time::local_civil(stored)?;
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
