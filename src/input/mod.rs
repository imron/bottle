pub mod cmd;
mod error;
mod tsv;

use crate::error::{Error, Fail, Usage};
use crate::ledger::{
    Agent, Amend, Clause, FieldInput, FieldValue, Get, Ignore, Last, List, Log, Op, Outcome,
    SchemaAdd, SchemaAddField, SchemaAddValue, SchemaDrop, SchemaRetire, SchemaShow, Sum, Today,
};
use crate::spec::{FieldName, Identifier, Link, LinkName, SchemaName, Spec, is_reserved};
use crate::time::{self, Period, Range};

pub use cmd::Cmd;

pub(crate) enum Style {
    Tsv,
    Yaml,
}

pub(crate) struct Request {
    pub op: Op,
    pub style: Style,
    pub show_ignored: bool,
}

pub(crate) fn parse(cmd: Cmd) -> Result<Request, Error> {
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
        op: Op::try_from(cmd)?,
        style,
        show_ignored,
    })
}

impl TryFrom<Cmd> for Op {
    type Error = Error;

    fn try_from(cmd: Cmd) -> Result<Self, Error> {
        Ok(match cmd {
            Cmd::Help(_) => return Err(Error::Fail(Fail::HelpNotAnOp)),
            Cmd::SchemaList => Op::SchemaList,
            Cmd::SchemaShow(cmd) => Op::SchemaShow(cmd.try_into()?),
            Cmd::SchemaAdd(cmd) => Op::SchemaAdd(cmd.try_into()?),
            Cmd::SchemaAddField(cmd) => Op::SchemaAddField(cmd.try_into()?),
            Cmd::SchemaAddValue(cmd) => Op::SchemaAddValue(cmd.try_into()?),
            Cmd::SchemaRetire(cmd) => Op::SchemaRetire(cmd.try_into()?),
            Cmd::SchemaDrop(cmd) => Op::SchemaDrop(cmd.try_into()?),
            Cmd::Log(cmd) => Op::Log(cmd.try_into()?),
            Cmd::Ls(cmd) => Op::List(cmd.try_into()?),
            Cmd::Get(cmd) => Op::Get(cmd.try_into()?),
            Cmd::Sum(cmd) => Op::Sum(cmd.try_into()?),
            Cmd::Last(cmd) => Op::Last(cmd.try_into()?),
            Cmd::Today(cmd) => Op::Today(cmd.try_into()?),
            Cmd::Amend(cmd) => Op::Amend(cmd.try_into()?),
            Cmd::Ignore(cmd) => Op::Ignore(cmd.try_into()?),
        })
    }
}

impl TryFrom<cmd::SchemaShow> for SchemaShow {
    type Error = Error;

    fn try_from(cmd: cmd::SchemaShow) -> Result<Self, Error> {
        Ok(Self {
            name: SchemaName::parse(&cmd.name)?,
        })
    }
}

impl TryFrom<cmd::SchemaAdd> for SchemaAdd {
    type Error = Error;

    fn try_from(cmd: cmd::SchemaAdd) -> Result<Self, Error> {
        let raw = std::fs::read_to_string(&cmd.file)?;
        Ok(Self {
            name: SchemaName::parse(&cmd.name)?,
            spec: Spec::parse_yaml(&raw)?,
        })
    }
}

impl TryFrom<cmd::SchemaAddField> for SchemaAddField {
    type Error = Error;

    fn try_from(cmd: cmd::SchemaAddField) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            name: FieldName::parse(&cmd.name)?,
            type_: cmd.type_,
            values: cmd.values,
            default: cmd.default,
        })
    }
}

impl TryFrom<cmd::SchemaAddValue> for SchemaAddValue {
    type Error = Error;

    fn try_from(cmd: cmd::SchemaAddValue) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            field: FieldName::parse(&cmd.field)?,
            value: cmd.value,
        })
    }
}

impl TryFrom<cmd::SchemaRetire> for SchemaRetire {
    type Error = Error;

    fn try_from(cmd: cmd::SchemaRetire) -> Result<Self, Error> {
        Ok(Self {
            name: SchemaName::parse(&cmd.name)?,
        })
    }
}

impl TryFrom<cmd::SchemaDrop> for SchemaDrop {
    type Error = Error;

    fn try_from(cmd: cmd::SchemaDrop) -> Result<Self, Error> {
        Ok(Self {
            name: SchemaName::parse(&cmd.name)?,
        })
    }
}

impl TryFrom<cmd::Log> for Log {
    type Error = Error;

    fn try_from(cmd: cmd::Log) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            at: cmd.at.as_deref().map(time::parse_instant).transpose()?,
            agent: cmd.agent.map(Agent::new),
            links: parse_links(cmd.links)?,
            fields: parse_fields(cmd.fields)?,
        })
    }
}

impl TryFrom<cmd::Ls> for List {
    type Error = Error;

    fn try_from(cmd: cmd::Ls) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            range: Range::parse(cmd.from.as_deref(), cmd.to.as_deref())?,
            agent: cmd.agent.map(Agent::new),
            filters: parse_clauses(cmd.wheres)?,
            include_ignored: cmd.include_ignored,
        })
    }
}

impl TryFrom<cmd::Get> for Get {
    type Error = Error;

    fn try_from(cmd: cmd::Get) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            id: cmd.id,
        })
    }
}

impl TryFrom<cmd::Sum> for Sum {
    type Error = Error;

    fn try_from(cmd: cmd::Sum) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            field: FieldName::parse(&cmd.field)?,
            range: Range::parse(cmd.from.as_deref(), cmd.to.as_deref())?,
            agent: cmd.agent.map(Agent::new),
            filters: parse_clauses(cmd.wheres)?,
            group: cmd
                .group
                .as_deref()
                .map(crate::spec::Group::parse)
                .transpose()?,
        })
    }
}

impl TryFrom<cmd::Last> for Last {
    type Error = Error;

    fn try_from(cmd: cmd::Last) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            agent: cmd.agent.map(Agent::new),
            filters: parse_clauses(cmd.wheres)?,
        })
    }
}

impl TryFrom<cmd::Today> for Today {
    type Error = Error;

    fn try_from(cmd: cmd::Today) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            agent: cmd.agent.map(Agent::new),
            filters: parse_clauses(cmd.wheres)?,
        })
    }
}

impl TryFrom<cmd::Amend> for Amend {
    type Error = Error;

    fn try_from(cmd: cmd::Amend) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            id: cmd.id,
            at: cmd.at.as_deref().map(time::parse_instant).transpose()?,
            agent: cmd.agent.map(Agent::new),
            links: parse_links(cmd.links)?,
            unlinks: parse_unlinks(cmd.unlinks)?,
            fields: parse_fields(cmd.fields)?,
        })
    }
}

impl TryFrom<cmd::Ignore> for Ignore {
    type Error = Error;

    fn try_from(cmd: cmd::Ignore) -> Result<Self, Error> {
        Ok(Self {
            schema: SchemaName::parse(&cmd.schema)?,
            id: cmd.id,
        })
    }
}

pub(crate) fn render(style: Style, show_ignored: bool, outcome: &Outcome) -> Result<String, Error> {
    match outcome {
        Outcome::Empty => Ok(String::new()),
        Outcome::Schemas(list) => {
            let rows: Vec<Vec<String>> = list
                .iter()
                .map(|s| vec![s.name.to_string(), tsv::bool_cell(s.retired).to_string()])
                .collect();
            Ok(tsv::table(&["name", "retired"], &rows))
        }
        Outcome::Spec(spec) => match style {
            Style::Yaml => spec.to_yaml(),
            Style::Tsv => render_spec(spec),
        },
        Outcome::Entries { spec, entries } => render_entries(spec, entries, show_ignored),
        Outcome::Posted { id, at, links } => {
            let at = time::display_local(*at)?;
            Ok(tsv::table(
                &["id", "at", "links"],
                &[vec![id.to_string(), at, render_links(links)]],
            ))
        }
        Outcome::Stamp { id, at } => {
            let at = time::display_local(*at)?;
            Ok(tsv::table(&["id", "at"], &[vec![id.to_string(), at]]))
        }
        Outcome::Total { field, value } => Ok(tsv::table(
            &["field", "value"],
            &[vec![field.to_string(), tsv::number(*value)]],
        )),
        Outcome::GroupedTime { unit, buckets } => {
            let rows: Vec<Vec<String>> = buckets
                .iter()
                .map(|(k, v)| vec![render_period(*k), tsv::number(*v)])
                .collect();
            Ok(tsv::table(&[unit.as_str(), "value"], &rows))
        }
        Outcome::GroupedLink { name, buckets } => {
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

fn parse_links(links: Vec<(String, String)>) -> Result<Vec<Link>, Error> {
    links
        .iter()
        .map(|(name, target)| Link::parse(name, target))
        .collect()
}

fn parse_unlinks(names: Vec<String>) -> Result<Vec<LinkName>, Error> {
    names.iter().map(|n| LinkName::parse(n)).collect()
}

fn parse_fields(fields: Vec<(String, String)>) -> Result<Vec<FieldInput>, Error> {
    fields
        .into_iter()
        .map(|(name, value)| {
            Ok(FieldInput {
                name: FieldName::parse(&name)?,
                value,
            })
        })
        .collect()
}

fn parse_clauses(wheres: Vec<(String, String)>) -> Result<Vec<Clause>, Error> {
    wheres
        .into_iter()
        .map(|(name, value)| {
            if is_reserved(&name) {
                return Err(Error::Usage(Usage::ReservedWhere(Identifier::parse(
                    &name,
                )?)));
            }
            Ok(Clause {
                name: Identifier::parse(&name)?,
                value,
            })
        })
        .collect()
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
        let values = match &field.values {
            Some(v) => v
                .iter()
                .map(crate::spec::EnumValue::as_str)
                .collect::<Vec<_>>()
                .join(","),
            None => String::new(),
        };
        rows.push(vec![
            field.name.to_string(),
            type_name(field.type_).to_string(),
            tsv::bool_cell(field.required).to_string(),
            values,
        ]);
    }
    Ok(tsv::table(&["name", "type", "required", "values"], &rows))
}

fn render_entries(
    spec: &Spec,
    entries: &[crate::ledger::Entry],
    show_ignored: bool,
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
            time::display_local(entry.at)?,
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

fn type_name(t: crate::spec::FieldType) -> &'static str {
    match t {
        crate::spec::FieldType::Text => "text",
        crate::spec::FieldType::Number => "number",
        crate::spec::FieldType::Enum => "enum",
    }
}
