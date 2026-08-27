pub mod cmd;
mod error;
pub mod help;
pub mod mcp;
mod tsv;

use std::collections::HashSet;
use std::path::PathBuf;

use jiff::tz::TimeZone;

use crate::error::{Error, Fail, Usage};
use crate::ledger::{
    Agent, Amend, Entries, Entry, FieldInput, FieldValue, Get, GroupedLink, GroupedTime, Ignore,
    Last, List, Log, Op, Outcome, Posted, SchemaAdd, SchemaAddField, SchemaAddValue, SchemaDrop,
    SchemaRetire, SchemaShow, Schemas, Scope, Stamp, Sum, Today, Total,
};
use crate::spec::{
    EnumValue, Field, FieldKind, FieldName, FieldType, FromTypeErr, Group, Identifier, Link,
    LinkName, SchemaName, Spec, is_reserved,
};
use crate::time::{self, Period, Range};

pub use cmd::Cmd;

#[derive(Debug, Clone, Copy)]
pub enum Style {
    Tsv,
    Yaml,
}

pub struct Request {
    pub op: Op,
    pub style: Style,
    pub show_ignored: bool,
}

impl Request {
    pub fn new(op: Op, style: Style) -> Self {
        Self {
            show_ignored: matches!(
                &op,
                Op::Get(_)
                    | Op::List(List {
                        include_ignored: true,
                        ..
                    })
            ),
            style,
            op,
        }
    }
}

pub enum SpecSource {
    File(PathBuf),
    Yaml(String),
}

pub struct ScopeInput {
    pub schema: String,
    pub agent: Option<String>,
    pub wheres: Vec<(String, String)>,
    pub links: Vec<(String, String)>,
}

impl From<cmd::Filters> for ScopeInput {
    fn from(filters: cmd::Filters) -> Self {
        Self {
            schema: filters.schema,
            agent: filters.agent,
            wheres: filters.wheres,
            links: filters.links,
        }
    }
}

pub struct AmendInput {
    pub schema: String,
    pub id: i64,
    pub at: Option<String>,
    pub agent: Option<String>,
    pub links: Vec<(String, String)>,
    pub unlinks: Vec<String>,
    pub fields: Vec<(String, String)>,
}

pub fn parse(cmd: Cmd, tz: &TimeZone) -> Result<Request, Error> {
    let style = match &cmd {
        Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow { yaml: true, .. })) => Style::Yaml,
        _ => Style::Tsv,
    };
    Ok(Request::new(op(cmd, tz)?, style))
}

fn op(cmd: Cmd, tz: &TimeZone) -> Result<Op, Error> {
    Ok(match cmd {
        Cmd::Schema(cmd::SchemaCmd::List) => Op::SchemaList,
        Cmd::Schema(cmd::SchemaCmd::Show(cmd)) => Op::SchemaShow(schema_show(cmd.name)?),
        Cmd::Schema(cmd::SchemaCmd::Add(cmd)) => {
            Op::SchemaAdd(schema_add(cmd.name, SpecSource::File(cmd.file))?)
        }
        Cmd::Schema(cmd::SchemaCmd::AddField(cmd)) => Op::SchemaAddField(schema_add_field(
            cmd.schema,
            cmd.name,
            cmd.type_,
            cmd.values,
            cmd.default,
        )?),
        Cmd::Schema(cmd::SchemaCmd::AddValue(cmd)) => {
            Op::SchemaAddValue(schema_add_value(cmd.schema, cmd.field, cmd.value)?)
        }
        Cmd::Schema(cmd::SchemaCmd::Retire(cmd)) => Op::SchemaRetire(schema_retire(cmd.name)?),
        Cmd::Schema(cmd::SchemaCmd::Drop(cmd)) => Op::SchemaDrop(schema_drop(cmd.name)?),
        Cmd::Log(cmd) => Op::Log(vec![log(
            cmd.schema, cmd.at, cmd.agent, cmd.links, cmd.fields, tz,
        )?]),
        Cmd::Logs(cmds) => {
            if cmds.is_empty() {
                return Err(Error::Usage(Usage::EmptyLog));
            }
            let mut ops = Vec::with_capacity(cmds.len());
            for cmd in cmds {
                ops.push(log(
                    cmd.schema, cmd.at, cmd.agent, cmd.links, cmd.fields, tz,
                )?);
            }
            Op::Log(ops)
        }
        Cmd::Ls(cmd) => Op::List(ls(
            cmd.filters.into(),
            cmd.from,
            cmd.to,
            cmd.include_ignored,
            tz,
        )?),
        Cmd::Get(cmd) => Op::Get(get(cmd.schema, cmd.id)?),
        Cmd::Sum(cmd) => Op::Sum(sum(
            cmd.filters.into(),
            cmd.field,
            cmd.from,
            cmd.to,
            cmd.group,
            tz,
        )?),
        Cmd::Last(cmd) => Op::Last(last(cmd.into())?),
        Cmd::Today(cmd) => Op::Today(today(cmd.into())?),
        Cmd::Amend(cmd) => Op::Amend(amend(
            AmendInput {
                schema: cmd.schema,
                id: cmd.id,
                at: cmd.at,
                agent: cmd.agent,
                links: cmd.links,
                unlinks: cmd.unlinks,
                fields: cmd.fields,
            },
            tz,
        )?),
        Cmd::Ignore(cmd) => Op::Ignore(ignore(cmd.schema, cmd.id)?),
    })
}

pub fn schema_show(name: String) -> Result<SchemaShow, Error> {
    Ok(SchemaShow {
        name: SchemaName::parse(&name)?,
    })
}

pub fn schema_add(name: String, source: SpecSource) -> Result<SchemaAdd, Error> {
    let raw = match source {
        SpecSource::File(path) => match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Fail(Fail::FileNotFound(path.display().to_string())));
            }
            Err(err) => return Err(err.into()),
        },
        SpecSource::Yaml(raw) => raw,
    };
    Ok(SchemaAdd {
        name: SchemaName::parse(&name)?,
        spec: Spec::parse_yaml(&raw)?,
    })
}

pub fn schema_add_field(
    schema: String,
    name: String,
    type_: FieldType,
    values: Option<Vec<String>>,
    default: Option<String>,
) -> Result<SchemaAddField, Error> {
    let name = FieldName::parse(&name)?;
    let kind = field_kind(type_, values)?;
    let default = match default.as_deref() {
        None => None,
        Some(raw) => Some(FieldValue::parse(
            &Field {
                name: name.clone(),
                kind: kind.clone(),
                required: true,
            },
            raw,
        )?),
    };
    Ok(SchemaAddField {
        schema: SchemaName::parse(&schema)?,
        name,
        kind,
        default,
    })
}

fn field_kind(type_: FieldType, values: Option<Vec<String>>) -> Result<FieldKind, Error> {
    let values = match (type_, values) {
        (FieldType::Enum, Some(raw)) => Some(
            raw.into_iter()
                .map(|s| EnumValue::parse(&s))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        (FieldType::Enum, None) => None,
        (_, None) => None,
        (_, Some(_)) => return Err(Error::Usage(Usage::EnumValuesNotAllowed)),
    };
    FieldKind::from_type(type_, values).map_err(|e| match e {
        FromTypeErr::ValuesRequired => Error::Usage(Usage::EnumValuesRequired),
        FromTypeErr::ValuesNotAllowed => Error::Usage(Usage::EnumValuesNotAllowed),
        FromTypeErr::Duplicate(v) => Error::Fail(Fail::DuplicateEnumValue(v)),
    })
}

pub fn schema_add_value(
    schema: String,
    field: String,
    value: String,
) -> Result<SchemaAddValue, Error> {
    Ok(SchemaAddValue {
        schema: SchemaName::parse(&schema)?,
        field: FieldName::parse(&field)?,
        value: EnumValue::parse(&value)?,
    })
}

pub fn schema_retire(name: String) -> Result<SchemaRetire, Error> {
    Ok(SchemaRetire {
        name: SchemaName::parse(&name)?,
    })
}

pub fn schema_drop(name: String) -> Result<SchemaDrop, Error> {
    Ok(SchemaDrop {
        name: SchemaName::parse(&name)?,
    })
}

pub fn log(
    schema: String,
    at: Option<String>,
    agent: Option<String>,
    links: Vec<(String, String)>,
    fields: Vec<(String, String)>,
    tz: &TimeZone,
) -> Result<Log, Error> {
    Ok(Log {
        schema: SchemaName::parse(&schema)?,
        at: at
            .as_deref()
            .map(|s| time::parse_instant(s, tz))
            .transpose()?,
        agent: parse_agent(agent)?,
        links: parse_links(links)?,
        fields: parse_fields(fields)?,
    })
}

pub fn ls(
    input: ScopeInput,
    from: Option<String>,
    to: Option<String>,
    include_ignored: bool,
    tz: &TimeZone,
) -> Result<List, Error> {
    Ok(List {
        scope: scope(input)?,
        range: Range::parse(from.as_deref(), to.as_deref(), tz)?,
        include_ignored,
    })
}

pub fn get(schema: String, id: i64) -> Result<Get, Error> {
    Ok(Get {
        schema: SchemaName::parse(&schema)?,
        id,
    })
}

pub fn sum(
    input: ScopeInput,
    field: String,
    from: Option<String>,
    to: Option<String>,
    group: Option<String>,
    tz: &TimeZone,
) -> Result<Sum, Error> {
    Ok(Sum {
        scope: scope(input)?,
        field: FieldName::parse(&field)?,
        range: Range::parse(from.as_deref(), to.as_deref(), tz)?,
        group: group.as_deref().map(Group::parse).transpose()?,
    })
}

pub fn last(input: ScopeInput) -> Result<Last, Error> {
    Ok(Last {
        scope: scope(input)?,
    })
}

pub fn today(input: ScopeInput) -> Result<Today, Error> {
    Ok(Today {
        scope: scope(input)?,
    })
}

fn scope(input: ScopeInput) -> Result<Scope, Error> {
    Ok(Scope {
        schema: SchemaName::parse(&input.schema)?,
        agent: parse_agent(input.agent)?,
        fields: parse_wheres(input.wheres)?,
        links: parse_links(input.links)?,
    })
}

pub fn amend(input: AmendInput, tz: &TimeZone) -> Result<Amend, Error> {
    let links = parse_links(input.links)?;
    let unlinks = parse_unlinks(input.unlinks)?;
    for name in &unlinks {
        if links.iter().any(|l| &l.name == name) {
            return Err(Error::Usage(Usage::LinkAndUnlink(name.clone())));
        }
    }
    let fields = parse_fields(input.fields)?;
    let at = input
        .at
        .as_deref()
        .map(|s| time::parse_instant(s, tz))
        .transpose()?;
    let agent = parse_agent(input.agent)?;
    if at.is_none()
        && agent.is_none()
        && links.is_empty()
        && unlinks.is_empty()
        && fields.is_empty()
    {
        return Err(Error::Usage(Usage::AmendEmpty));
    }
    Ok(Amend {
        schema: SchemaName::parse(&input.schema)?,
        id: input.id,
        at,
        agent,
        links,
        unlinks,
        fields,
    })
}

pub fn ignore(schema: String, id: i64) -> Result<Ignore, Error> {
    Ok(Ignore {
        schema: SchemaName::parse(&schema)?,
        id,
    })
}

pub fn render(
    style: Style,
    show_ignored: bool,
    outcome: &Outcome,
    tz: &TimeZone,
) -> Result<String, Error> {
    match outcome {
        Outcome::Empty => Ok(String::new()),
        Outcome::Schemas(Schemas { schemas }) => {
            let rows: Vec<Vec<String>> = schemas
                .iter()
                .map(|s| vec![s.name.to_string(), tsv::bool_cell(s.retired).to_string()])
                .collect();
            Ok(tsv::table(&["name", "retired"], &rows))
        }
        Outcome::Spec(spec) => match style {
            Style::Yaml => spec.to_yaml(),
            Style::Tsv => render_spec(spec),
        },
        Outcome::Entries(Entries { spec, entries }) => {
            render_entries(spec, entries, show_ignored, tz)
        }
        Outcome::Posted(rows) => {
            let mut out = Vec::new();
            for Posted { id, at, links } in rows {
                out.push(vec![
                    id.to_string(),
                    time::display_local(*at, tz)?,
                    render_links(links),
                ]);
            }
            Ok(tsv::table(&["id", "at", "links"], &out))
        }
        Outcome::Stamp(Stamp { id, at }) => {
            let at = time::display_local(*at, tz)?;
            Ok(tsv::table(&["id", "at"], &[vec![id.to_string(), at]]))
        }
        Outcome::Total(Total { field, value }) => Ok(tsv::table(
            &["field", "value"],
            &[vec![field.to_string(), tsv::number(*value)]],
        )),
        Outcome::GroupedTime(GroupedTime { unit, buckets }) => {
            let rows: Vec<Vec<String>> = buckets
                .iter()
                .map(|(k, v)| vec![render_period(*k), tsv::number(*v)])
                .collect();
            Ok(tsv::table(&[unit.as_str(), "value"], &rows))
        }
        Outcome::GroupedLink(GroupedLink { name, buckets }) => {
            let rows: Vec<Vec<String>> = buckets
                .iter()
                .map(|(k, v)| {
                    vec![
                        k.as_ref().map(ToString::to_string).unwrap_or_default(),
                        tsv::number(*v),
                    ]
                })
                .collect();
            Ok(tsv::table(&[name.as_str(), "value"], &rows))
        }
    }
}

fn parse_agent(agent: Option<String>) -> Result<Option<Agent>, Error> {
    agent.as_deref().map(Agent::parse).transpose()
}

fn parse_links(links: Vec<(String, String)>) -> Result<Vec<Link>, Error> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (name, target) in links {
        let link = Link::parse(&name, &target)?;
        if !seen.insert(link.name.clone()) {
            return Err(Error::Usage(Usage::DuplicateLinkName(link.name)));
        }
        out.push(link);
    }
    Ok(out)
}

fn parse_unlinks(names: Vec<String>) -> Result<Vec<LinkName>, Error> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for name in names {
        let name = LinkName::parse(&name)?;
        if !seen.insert(name.clone()) {
            return Err(Error::Usage(Usage::DuplicateUnlink(name)));
        }
        out.push(name);
    }
    Ok(out)
}

fn parse_fields(fields: Vec<(String, String)>) -> Result<Vec<FieldInput>, Error> {
    named_fields(fields, false)
}

fn parse_wheres(wheres: Vec<(String, String)>) -> Result<Vec<FieldInput>, Error> {
    named_fields(wheres, true)
}

fn named_fields(
    pairs: Vec<(String, String)>,
    reject_reserved: bool,
) -> Result<Vec<FieldInput>, Error> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (name, value) in pairs {
        if reject_reserved && is_reserved(&name) {
            return Err(Error::Usage(Usage::ReservedWhere(Identifier::parse(
                &name,
            )?)));
        }
        let name = FieldName::parse(&name)?;
        if !seen.insert(name.clone()) {
            return Err(Error::Usage(Usage::DuplicateField(name)));
        }
        out.push(FieldInput { name, value });
    }
    Ok(out)
}

fn render_period(period: Period) -> String {
    match period {
        Period::Day(date) => date.to_string(),
        Period::Week { year, week } => format!("{year}-W{week:02}"),
        Period::Month { year, month } => format!("{year:04}-{month:02}"),
        Period::Year(year) => format!("{year:04}"),
    }
}

fn render_spec(spec: &Spec) -> Result<String, Error> {
    let mut rows = Vec::new();
    for field in &spec.fields {
        let values = match &field.kind {
            FieldKind::Enum(v) => v
                .iter()
                .map(EnumValue::as_str)
                .collect::<Vec<_>>()
                .join(","),
            _ => String::new(),
        };
        rows.push(vec![
            field.name.to_string(),
            type_name(&field.kind).to_string(),
            tsv::bool_cell(field.required).to_string(),
            values,
        ]);
    }
    Ok(tsv::table(&["name", "type", "required", "values"], &rows))
}

fn render_entries(
    spec: &Spec,
    entries: &[Entry],
    show_ignored: bool,
    tz: &TimeZone,
) -> Result<String, Error> {
    let mut headers = vec!["id".to_string(), "at".to_string(), "links".to_string()];
    for field in &spec.fields {
        headers.push(field.name.to_string());
    }
    headers.push("agent".to_string());
    if show_ignored {
        headers.push("ignored".to_string());
    }
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let mut out_rows = Vec::new();
    for entry in entries {
        let mut cells = vec![
            entry.id.to_string(),
            time::display_local(entry.at, tz)?,
            render_links(&entry.links),
        ];
        for field in &spec.fields {
            cells.push(render_value(entry.values.get(&field.name)));
        }
        cells.push(
            entry
                .agent
                .as_ref()
                .map(Agent::as_str)
                .unwrap_or_default()
                .to_string(),
        );
        if show_ignored {
            cells.push(tsv::bool_cell(entry.ignored).to_string());
        }
        out_rows.push(cells);
    }
    Ok(tsv::table(&header_refs, &out_rows))
}

fn render_value(value: Option<&FieldValue>) -> String {
    match value {
        None | Some(FieldValue::Empty) => String::new(),
        Some(FieldValue::Text(s)) => s.clone(),
        Some(FieldValue::Number(n)) => tsv::number(*n),
        Some(FieldValue::Enum(v)) => v.to_string(),
    }
}

fn render_links(links: &[Link]) -> String {
    links
        .iter()
        .map(|l| format!("{}={}", l.name, l.to))
        .collect::<Vec<_>>()
        .join(" ")
}

fn type_name(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        FieldKind::Number => "number",
        FieldKind::Enum(_) => "enum",
    }
}
