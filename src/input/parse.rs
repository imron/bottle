use std::collections::HashSet;

use jiff::tz::TimeZone;

use crate::error::{Error, Fail, Usage};
use crate::ledger::{
    Agent, Amend, FieldInput, FieldValue, Get, Ignore, Last, List, Log, Op, SchemaAdd,
    SchemaAddField, SchemaAddValue, SchemaDrop, SchemaRetire, SchemaShow, Scope, Style, Sum, Today,
};
use crate::spec::{
    EntryId, EnumValue, Field, FieldKind, FieldName, FieldType, FromTypeErr, Group, Identifier,
    Link, LinkName, SchemaName, Spec, is_reserved,
};
use crate::time::{self, Range};

use super::{AmendInput, Cmd, ScopeInput, SpecSource, cmd};

pub fn parse(cmd: Cmd, tz: &TimeZone) -> Result<Op, Error> {
    Ok(match cmd {
        Cmd::Schema(cmd::SchemaCmd::List) => Op::SchemaList,
        Cmd::Schema(cmd::SchemaCmd::Show(cmd)) => {
            let style = if cmd.yaml { Style::Yaml } else { Style::Tsv };
            Op::SchemaShow(schema_show(cmd.name, style)?)
        }
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

pub fn schema_show(name: String, style: Style) -> Result<SchemaShow, Error> {
    Ok(SchemaShow {
        name: SchemaName::parse(&name)?,
        style,
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
        id: EntryId::parse(id)?,
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
        id: EntryId::parse(input.id)?,
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
        id: EntryId::parse(id)?,
    })
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
