mod cmd;
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
        Cmd::SchemaShow { yaml: true, .. } => Style::Yaml,
        _ => Style::Tsv,
    };
    let show_ignored = matches!(
        &cmd,
        Cmd::Get { .. }
            | Cmd::Ls {
                include_ignored: true,
                ..
            }
    );
    let op = match cmd {
        Cmd::Help { .. } => {
            return Err(Error::Fail(Fail::HelpNotAnOp));
        }
        Cmd::SchemaList => Op::SchemaList,
        Cmd::SchemaShow { name, yaml: _ } => Op::SchemaShow(SchemaShow {
            name: SchemaName::parse(&name)?,
        }),
        Cmd::SchemaAdd { name, file } => {
            let raw = std::fs::read_to_string(&file)?;
            Op::SchemaAdd(SchemaAdd {
                name: SchemaName::parse(&name)?,
                spec: Spec::parse_yaml(&raw)?,
            })
        }
        Cmd::SchemaAddField {
            schema,
            name,
            type_,
            values,
            default,
        } => Op::SchemaAddField(SchemaAddField {
            schema: SchemaName::parse(&schema)?,
            name: FieldName::parse(&name)?,
            type_,
            values,
            default,
        }),
        Cmd::SchemaAddValue {
            schema,
            field,
            value,
        } => Op::SchemaAddValue(SchemaAddValue {
            schema: SchemaName::parse(&schema)?,
            field: FieldName::parse(&field)?,
            value,
        }),
        Cmd::SchemaRetire { name } => Op::SchemaRetire(SchemaRetire {
            name: SchemaName::parse(&name)?,
        }),
        Cmd::SchemaDrop { name } => Op::SchemaDrop(SchemaDrop {
            name: SchemaName::parse(&name)?,
        }),
        Cmd::Log {
            schema,
            at,
            agent,
            links,
            fields,
        } => Op::Log(Log {
            schema: SchemaName::parse(&schema)?,
            at: at.as_deref().map(time::parse_instant).transpose()?,
            agent: agent.map(Agent::new),
            links: parse_links(links)?,
            fields: parse_fields(fields)?,
        }),
        Cmd::Ls {
            schema,
            from,
            to,
            agent,
            wheres,
            include_ignored,
        } => Op::List(List {
            schema: SchemaName::parse(&schema)?,
            range: Range::parse(from.as_deref(), to.as_deref())?,
            agent: agent.map(Agent::new),
            filters: parse_clauses(wheres)?,
            include_ignored,
        }),
        Cmd::Get { schema, id } => Op::Get(Get {
            schema: SchemaName::parse(&schema)?,
            id,
        }),
        Cmd::Sum {
            schema,
            field,
            from,
            to,
            agent,
            wheres,
            group,
        } => Op::Sum(Sum {
            schema: SchemaName::parse(&schema)?,
            field: FieldName::parse(&field)?,
            range: Range::parse(from.as_deref(), to.as_deref())?,
            agent: agent.map(Agent::new),
            filters: parse_clauses(wheres)?,
            group: group
                .as_deref()
                .map(crate::spec::Group::parse)
                .transpose()?,
        }),
        Cmd::Last {
            schema,
            agent,
            wheres,
        } => Op::Last(Last {
            schema: SchemaName::parse(&schema)?,
            agent: agent.map(Agent::new),
            filters: parse_clauses(wheres)?,
        }),
        Cmd::Today {
            schema,
            agent,
            wheres,
        } => Op::Today(Today {
            schema: SchemaName::parse(&schema)?,
            agent: agent.map(Agent::new),
            filters: parse_clauses(wheres)?,
        }),
        Cmd::Amend {
            schema,
            id,
            at,
            agent,
            links,
            unlinks,
            fields,
        } => Op::Amend(Amend {
            schema: SchemaName::parse(&schema)?,
            id,
            at: at.as_deref().map(time::parse_instant).transpose()?,
            agent: agent.map(Agent::new),
            links: parse_links(links)?,
            unlinks: parse_unlinks(unlinks)?,
            fields: parse_fields(fields)?,
        }),
        Cmd::Ignore { schema, id } => Op::Ignore(Ignore {
            schema: SchemaName::parse(&schema)?,
            id,
        }),
    };
    Ok(Request {
        op,
        style,
        show_ignored,
    })
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
