pub mod cmd;
mod error;
pub(crate) mod help;
mod tsv;

use std::collections::HashSet;

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

pub enum Style {
    Tsv,
    Yaml,
}

pub struct Request {
    pub op: Op,
    pub style: Style,
    pub show_ignored: bool,
}

pub fn parse(cmd: Cmd, tz: &TimeZone) -> Result<Request, Error> {
    let style = match &cmd {
        Cmd::SchemaShow(cmd::SchemaShow { yaml: true, .. }) => Style::Yaml,
        _ => Style::Tsv,
    };
    let show_ignored = matches!(
        &cmd,
        Cmd::Get(_)
            | Cmd::Ls(cmd::Ls {
                include_ignored: true,
                ..
            })
    );
    Ok(Request {
        op: op(cmd, tz)?,
        style,
        show_ignored,
    })
}

fn op(cmd: Cmd, tz: &TimeZone) -> Result<Op, Error> {
    Ok(match cmd {
        Cmd::SchemaList => Op::SchemaList,
        Cmd::SchemaShow(cmd) => Op::SchemaShow(schema_show(cmd)?),
        Cmd::SchemaAdd(cmd) => Op::SchemaAdd(schema_add(cmd)?),
        Cmd::SchemaAddField(cmd) => Op::SchemaAddField(schema_add_field(cmd)?),
        Cmd::SchemaAddValue(cmd) => Op::SchemaAddValue(schema_add_value(cmd)?),
        Cmd::SchemaRetire(cmd) => Op::SchemaRetire(schema_retire(cmd)?),
        Cmd::SchemaDrop(cmd) => Op::SchemaDrop(schema_drop(cmd)?),
        Cmd::Log(cmd) => Op::Log(log(cmd, tz)?),
        Cmd::Ls(cmd) => Op::List(ls(cmd, tz)?),
        Cmd::Get(cmd) => Op::Get(get(cmd)?),
        Cmd::Sum(cmd) => Op::Sum(sum(cmd, tz)?),
        Cmd::Last(cmd) => Op::Last(last(cmd)?),
        Cmd::Today(cmd) => Op::Today(today(cmd)?),
        Cmd::Amend(cmd) => Op::Amend(amend(cmd, tz)?),
        Cmd::Ignore(cmd) => Op::Ignore(ignore(cmd)?),
    })
}

fn schema_show(cmd: cmd::SchemaShow) -> Result<SchemaShow, Error> {
    Ok(SchemaShow {
        name: SchemaName::parse(&cmd.name)?,
    })
}

fn schema_add(cmd: cmd::SchemaAdd) -> Result<SchemaAdd, Error> {
    let raw = match std::fs::read_to_string(&cmd.file) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::Fail(Fail::FileNotFound(
                cmd.file.display().to_string(),
            )));
        }
        Err(err) => return Err(err.into()),
    };
    Ok(SchemaAdd {
        name: SchemaName::parse(&cmd.name)?,
        spec: Spec::parse_yaml(&raw)?,
    })
}

fn schema_add_field(cmd: cmd::SchemaAddField) -> Result<SchemaAddField, Error> {
    let name = FieldName::parse(&cmd.name)?;
    let kind = field_kind(cmd.type_, cmd.values)?;
    let field = Field {
        name,
        kind,
        required: cmd.default.is_some(),
    };
    let default = cmd
        .default
        .as_deref()
        .map(|raw| FieldValue::parse(&field, raw))
        .transpose()?;
    Ok(SchemaAddField {
        schema: SchemaName::parse(&cmd.schema)?,
        field,
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

fn schema_add_value(cmd: cmd::SchemaAddValue) -> Result<SchemaAddValue, Error> {
    Ok(SchemaAddValue {
        schema: SchemaName::parse(&cmd.schema)?,
        field: FieldName::parse(&cmd.field)?,
        value: EnumValue::parse(&cmd.value)?,
    })
}

fn schema_retire(cmd: cmd::SchemaRetire) -> Result<SchemaRetire, Error> {
    Ok(SchemaRetire {
        name: SchemaName::parse(&cmd.name)?,
    })
}

fn schema_drop(cmd: cmd::SchemaDrop) -> Result<SchemaDrop, Error> {
    Ok(SchemaDrop {
        name: SchemaName::parse(&cmd.name)?,
    })
}

pub(crate) fn log(cmd: cmd::Log, tz: &TimeZone) -> Result<Log, Error> {
    Ok(Log {
        schema: SchemaName::parse(&cmd.schema)?,
        at: cmd
            .at
            .as_deref()
            .map(|s| time::parse_instant(s, tz))
            .transpose()?,
        agent: parse_agent(cmd.agent)?,
        links: parse_links(cmd.links)?,
        fields: parse_fields(cmd.fields)?,
    })
}

fn ls(cmd: cmd::Ls, tz: &TimeZone) -> Result<List, Error> {
    Ok(List {
        scope: scope(cmd.schema, cmd.agent, cmd.wheres, cmd.links)?,
        range: Range::parse(cmd.from.as_deref(), cmd.to.as_deref(), tz)?,
        include_ignored: cmd.include_ignored,
    })
}

fn get(cmd: cmd::Get) -> Result<Get, Error> {
    Ok(Get {
        schema: SchemaName::parse(&cmd.schema)?,
        id: cmd.id,
    })
}

fn sum(cmd: cmd::Sum, tz: &TimeZone) -> Result<Sum, Error> {
    Ok(Sum {
        scope: scope(cmd.schema, cmd.agent, cmd.wheres, cmd.links)?,
        field: FieldName::parse(&cmd.field)?,
        range: Range::parse(cmd.from.as_deref(), cmd.to.as_deref(), tz)?,
        group: cmd.group.as_deref().map(Group::parse).transpose()?,
    })
}

fn last(cmd: cmd::Last) -> Result<Last, Error> {
    Ok(Last {
        scope: scope(cmd.schema, cmd.agent, cmd.wheres, cmd.links)?,
    })
}

fn today(cmd: cmd::Today) -> Result<Today, Error> {
    Ok(Today {
        scope: scope(cmd.schema, cmd.agent, cmd.wheres, cmd.links)?,
    })
}

fn scope(
    schema: String,
    agent: Option<String>,
    wheres: Vec<(String, String)>,
    links: Vec<(String, String)>,
) -> Result<Scope, Error> {
    Ok(Scope {
        schema: SchemaName::parse(&schema)?,
        agent: parse_agent(agent)?,
        fields: parse_wheres(wheres)?,
        links: parse_links(links)?,
    })
}

fn amend(cmd: cmd::Amend, tz: &TimeZone) -> Result<Amend, Error> {
    let links = parse_links(cmd.links)?;
    let unlinks = parse_unlinks(cmd.unlinks)?;
    for name in &unlinks {
        if links.iter().any(|l| &l.name == name) {
            return Err(Error::Usage(Usage::LinkAndUnlink(name.clone())));
        }
    }
    let fields = parse_fields(cmd.fields)?;
    let at = cmd
        .at
        .as_deref()
        .map(|s| time::parse_instant(s, tz))
        .transpose()?;
    let agent = parse_agent(cmd.agent)?;
    if at.is_none()
        && agent.is_none()
        && links.is_empty()
        && unlinks.is_empty()
        && fields.is_empty()
    {
        return Err(Error::Usage(Usage::AmendEmpty));
    }
    Ok(Amend {
        schema: SchemaName::parse(&cmd.schema)?,
        id: cmd.id,
        at,
        agent,
        links,
        unlinks,
        fields,
    })
}

fn ignore(cmd: cmd::Ignore) -> Result<Ignore, Error> {
    Ok(Ignore {
        schema: SchemaName::parse(&cmd.schema)?,
        id: cmd.id,
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
