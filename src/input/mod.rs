pub mod cmd;
mod error;
pub(crate) mod help;
mod tsv;

use jiff::tz::TimeZone;

use crate::error::{Error, Fail, Usage};
use crate::ledger::{
    Agent, Amend, Entries, FieldInput, FieldValue, Get, GroupedLink, GroupedTime, Ignore, Last,
    List, Log, Op, Outcome, Posted, SchemaAdd, SchemaAddField, SchemaAddValue, SchemaDrop,
    SchemaRetire, SchemaShow, Schemas, Stamp, Sum, Today, Total,
};
use crate::spec::{FieldName, Identifier, Link, LinkName, SchemaName, Spec, is_reserved};
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
        op: cmd.try_from(tz)?,
        style,
        show_ignored,
    })
}

trait FromCmd {
    type Op;
    fn try_from(self, tz: &TimeZone) -> Result<Self::Op, Error>;
}

impl FromCmd for Cmd {
    type Op = Op;

    fn try_from(self, tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(match self {
            Cmd::SchemaList => Op::SchemaList,
            Cmd::SchemaShow(cmd) => Op::SchemaShow(cmd.try_from(tz)?),
            Cmd::SchemaAdd(cmd) => Op::SchemaAdd(cmd.try_from(tz)?),
            Cmd::SchemaAddField(cmd) => Op::SchemaAddField(cmd.try_from(tz)?),
            Cmd::SchemaAddValue(cmd) => Op::SchemaAddValue(cmd.try_from(tz)?),
            Cmd::SchemaRetire(cmd) => Op::SchemaRetire(cmd.try_from(tz)?),
            Cmd::SchemaDrop(cmd) => Op::SchemaDrop(cmd.try_from(tz)?),
            Cmd::Log(cmd) => Op::Log(cmd.try_from(tz)?),
            Cmd::Ls(cmd) => Op::List(cmd.try_from(tz)?),
            Cmd::Get(cmd) => Op::Get(cmd.try_from(tz)?),
            Cmd::Sum(cmd) => Op::Sum(cmd.try_from(tz)?),
            Cmd::Last(cmd) => Op::Last(cmd.try_from(tz)?),
            Cmd::Today(cmd) => Op::Today(cmd.try_from(tz)?),
            Cmd::Amend(cmd) => Op::Amend(cmd.try_from(tz)?),
            Cmd::Ignore(cmd) => Op::Ignore(cmd.try_from(tz)?),
        })
    }
}

impl FromCmd for cmd::SchemaShow {
    type Op = SchemaShow;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(SchemaShow {
            name: SchemaName::parse(&self.name)?,
        })
    }
}

impl FromCmd for cmd::SchemaAdd {
    type Op = SchemaAdd;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        let raw = match std::fs::read_to_string(&self.file) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Fail(Fail::FileNotFound(
                    self.file.display().to_string(),
                )));
            }
            Err(err) => return Err(err.into()),
        };
        Ok(SchemaAdd {
            name: SchemaName::parse(&self.name)?,
            spec: Spec::parse_yaml(&raw)?,
        })
    }
}

impl FromCmd for cmd::SchemaAddField {
    type Op = SchemaAddField;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(SchemaAddField {
            schema: SchemaName::parse(&self.schema)?,
            name: FieldName::parse(&self.name)?,
            type_: self.type_,
            values: self.values,
            default: self.default,
        })
    }
}

impl FromCmd for cmd::SchemaAddValue {
    type Op = SchemaAddValue;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(SchemaAddValue {
            schema: SchemaName::parse(&self.schema)?,
            field: FieldName::parse(&self.field)?,
            value: self.value,
        })
    }
}

impl FromCmd for cmd::SchemaRetire {
    type Op = SchemaRetire;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(SchemaRetire {
            name: SchemaName::parse(&self.name)?,
        })
    }
}

impl FromCmd for cmd::SchemaDrop {
    type Op = SchemaDrop;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(SchemaDrop {
            name: SchemaName::parse(&self.name)?,
        })
    }
}

impl FromCmd for cmd::Log {
    type Op = Log;

    fn try_from(self, tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(Log {
            schema: SchemaName::parse(&self.schema)?,
            at: self
                .at
                .as_deref()
                .map(|s| time::parse_instant(s, tz))
                .transpose()?,
            agent: parse_agent(self.agent)?,
            links: parse_links(self.links)?,
            fields: parse_fields(self.fields)?,
        })
    }
}

impl FromCmd for cmd::Ls {
    type Op = List;

    fn try_from(self, tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(List {
            schema: SchemaName::parse(&self.schema)?,
            range: Range::parse(self.from.as_deref(), self.to.as_deref(), tz)?,
            agent: parse_agent(self.agent)?,
            fields: parse_wheres(self.wheres)?,
            links: parse_links(self.links)?,
            include_ignored: self.include_ignored,
        })
    }
}

impl FromCmd for cmd::Get {
    type Op = Get;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(Get {
            schema: SchemaName::parse(&self.schema)?,
            id: self.id,
        })
    }
}

impl FromCmd for cmd::Sum {
    type Op = Sum;

    fn try_from(self, tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(Sum {
            schema: SchemaName::parse(&self.schema)?,
            field: FieldName::parse(&self.field)?,
            range: Range::parse(self.from.as_deref(), self.to.as_deref(), tz)?,
            agent: parse_agent(self.agent)?,
            fields: parse_wheres(self.wheres)?,
            links: parse_links(self.links)?,
            group: self
                .group
                .as_deref()
                .map(crate::spec::Group::parse)
                .transpose()?,
        })
    }
}

impl FromCmd for cmd::Last {
    type Op = Last;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(Last {
            schema: SchemaName::parse(&self.schema)?,
            agent: parse_agent(self.agent)?,
            fields: parse_wheres(self.wheres)?,
            links: parse_links(self.links)?,
        })
    }
}

impl FromCmd for cmd::Today {
    type Op = Today;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(Today {
            schema: SchemaName::parse(&self.schema)?,
            agent: parse_agent(self.agent)?,
            fields: parse_wheres(self.wheres)?,
            links: parse_links(self.links)?,
        })
    }
}

impl FromCmd for cmd::Amend {
    type Op = Amend;

    fn try_from(self, tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(Amend {
            schema: SchemaName::parse(&self.schema)?,
            id: self.id,
            at: self
                .at
                .as_deref()
                .map(|s| time::parse_instant(s, tz))
                .transpose()?,
            agent: parse_agent(self.agent)?,
            links: parse_links(self.links)?,
            unlinks: parse_unlinks(self.unlinks)?,
            fields: parse_fields(self.fields)?,
        })
    }
}

impl FromCmd for cmd::Ignore {
    type Op = Ignore;

    fn try_from(self, _tz: &TimeZone) -> Result<Self::Op, Error> {
        Ok(Ignore {
            schema: SchemaName::parse(&self.schema)?,
            id: self.id,
        })
    }
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
        Outcome::Posted(Posted { id, at, links }) => {
            let at = time::display_local(*at, tz)?;
            Ok(tsv::table(
                &["id", "at", "links"],
                &[vec![id.to_string(), at, render_links(links)]],
            ))
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

fn parse_wheres(wheres: Vec<(String, String)>) -> Result<Vec<FieldInput>, Error> {
    wheres
        .into_iter()
        .map(|(name, value)| {
            if is_reserved(&name) {
                return Err(Error::Usage(Usage::ReservedWhere(Identifier::parse(
                    &name,
                )?)));
            }
            Ok(FieldInput {
                name: FieldName::parse(&name)?,
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

fn type_name(t: crate::spec::FieldType) -> &'static str {
    match t {
        crate::spec::FieldType::Text => "text",
        crate::spec::FieldType::Number => "number",
        crate::spec::FieldType::Enum => "enum",
    }
}
