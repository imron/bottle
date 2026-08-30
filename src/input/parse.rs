use std::collections::HashSet;
use std::path::{Path, PathBuf};

use jiff::tz::TimeZone;

use crate::error::{Error, Fail, Usage};
use crate::ledger::{
    Agent, Amend, Backup, FieldInput, Get, Ignore, Last, List, Log, NonEmptyFieldValue, Op,
    SchemaAdd, SchemaAddField, SchemaAddValue, SchemaDrop, SchemaRename, SchemaRenameField,
    SchemaRetire, SchemaShow, Scope, Sum, Today, Unignore,
};
use crate::spec::{
    EntryId, EnumValue, Field, FieldKind, FieldName, FieldType, FromTypeErr, Group, Identifier,
    Link, LinkName, SchemaName, Spec,
};
use crate::time::{self, Range};

use super::{AmendInput, Cmd, LogInput, ScopeInput, SpecSource, cmd};

pub fn parse(cmd: Cmd, tz: &TimeZone) -> Result<Op, Error> {
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
        Cmd::Schema(cmd::SchemaCmd::Rename(cmd)) => {
            Op::SchemaRename(schema_rename(cmd.from, cmd.to)?)
        }
        Cmd::Schema(cmd::SchemaCmd::RenameField(cmd)) => {
            Op::SchemaRenameField(schema_rename_field(cmd.schema, cmd.from, cmd.to)?)
        }
        Cmd::Schema(cmd::SchemaCmd::Retire(cmd)) => Op::SchemaRetire(schema_retire(cmd.name)?),
        Cmd::Schema(cmd::SchemaCmd::Drop(cmd)) => Op::SchemaDrop(schema_drop(cmd.name)?),
        Cmd::Log(cmd) => logs(log_cmd(cmd)?, tz)?,
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
        Cmd::Unignore(cmd) => Op::Unignore(unignore(cmd.schema, cmd.id)?),
        Cmd::Backup(cmd) => Op::Backup(backup(cmd.path)?),
    })
}

pub fn schema_show(name: String) -> Result<SchemaShow, Error> {
    Ok(SchemaShow {
        name: SchemaName::parse(&name)?,
    })
}

pub fn schema_add(name: String, source: SpecSource) -> Result<SchemaAdd, Error> {
    let raw = match source {
        SpecSource::File(path) => read_text(&path)?,
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
        Some(raw) => Some(NonEmptyFieldValue::parse(
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
    if type_ != FieldType::Enum && values.is_some() {
        return Err(Error::Usage(Usage::EnumValuesNotAllowed));
    }
    let values = values
        .map(|raw| {
            raw.into_iter()
                .map(|s| EnumValue::parse(&s))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    FieldKind::from_type(type_, values).map_err(|e| match e {
        FromTypeErr::Duplicate(v) => Error::Fail(Fail::DuplicateEnumValue(v)),
        FromTypeErr::ValuesRequired | FromTypeErr::ValuesNotAllowed => {
            Error::Usage(Usage::EnumValuesRequired)
        }
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

pub fn schema_rename(from: String, to: String) -> Result<SchemaRename, Error> {
    let from = SchemaName::parse(&from)?;
    let to = SchemaName::parse(&to)?;
    if from == to {
        return Err(Error::Usage(Usage::RenameSameSchema(from)));
    }
    Ok(SchemaRename { from, to })
}

pub fn schema_rename_field(
    schema: String,
    from: String,
    to: String,
) -> Result<SchemaRenameField, Error> {
    let from = FieldName::parse(&from)?;
    let to = FieldName::parse(&to)?;
    if from == to {
        return Err(Error::Usage(Usage::RenameSameField(from)));
    }
    Ok(SchemaRenameField {
        schema: SchemaName::parse(&schema)?,
        from,
        to,
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

fn log_cmd(cmd: cmd::Log) -> Result<Vec<LogInput>, Error> {
    match cmd.file {
        None => Ok(vec![cmd.into()]),
        Some(ref path) => {
            let raw = read_text(path)?;
            let mut rows = super::tsv::log_rows(&cmd.schema, &raw)?;
            for row in &mut rows {
                apply_log_defaults(row, &cmd);
            }
            Ok(rows)
        }
    }
}

fn apply_log_defaults(row: &mut LogInput, cmd: &cmd::Log) {
    if row.at.is_none() {
        row.at.clone_from(&cmd.at);
    }
    if row.agent.is_none() {
        row.agent.clone_from(&cmd.agent);
    }
    row.links = overlay_pairs(&cmd.links, &row.links);
    let present: HashSet<String> = row.fields.iter().map(|(n, _)| n.clone()).collect();
    for (name, value) in &cmd.fields {
        if !present.contains(name) {
            row.fields.push((name.clone(), value.clone()));
        }
    }
}

fn overlay_pairs(
    defaults: &[(String, String)],
    overlay: &[(String, String)],
) -> Vec<(String, String)> {
    let mut out = defaults.to_vec();
    for (name, value) in overlay {
        if let Some((_, existing)) = out.iter_mut().find(|(n, _)| n == name) {
            *existing = value.clone();
        } else {
            out.push((name.clone(), value.clone()));
        }
    }
    out
}

fn read_text(path: &Path) -> Result<String, Error> {
    if path.as_os_str() == "-" {
        let mut raw = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut raw)?;
        return Ok(raw);
    }
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(raw),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(Error::Fail(Fail::FileNotFound(path.display().to_string())))
        }
        Err(err) => Err(err.into()),
    }
}

pub fn logs(entries: Vec<LogInput>, tz: &TimeZone) -> Result<Op, Error> {
    if entries.is_empty() {
        return Err(Error::Usage(Usage::EmptyLog));
    }
    let mut ops = Vec::with_capacity(entries.len());
    for entry in entries {
        ops.push(log(entry, tz)?);
    }
    Ok(Op::Log(ops))
}

fn log(entry: LogInput, tz: &TimeZone) -> Result<Log, Error> {
    Ok(Log {
        schema: SchemaName::parse(&entry.schema)?,
        at: entry
            .at
            .as_deref()
            .map(|s| time::parse_at(s, tz))
            .transpose()?,
        agent: parse_agent(entry.agent)?,
        links: parse_links(entry.links)?,
        fields: parse_fields(entry.fields)?,
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

pub fn get(schema: String, id: EntryId) -> Result<Get, Error> {
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
        excludes: parse_excludes(input.excludes)?,
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
        .map(|s| time::parse_at(s, tz))
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

pub fn ignore(schema: String, id: EntryId) -> Result<Ignore, Error> {
    Ok(Ignore {
        schema: SchemaName::parse(&schema)?,
        id,
    })
}

pub fn unignore(schema: String, id: EntryId) -> Result<Unignore, Error> {
    Ok(Unignore {
        schema: SchemaName::parse(&schema)?,
        id,
    })
}

pub fn backup(path: PathBuf) -> Result<Backup, Error> {
    if path.as_os_str().is_empty() {
        return Err(Error::Usage(Usage::EmptyBackupPath));
    }
    Ok(Backup { path })
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
    named_fields(fields, false, true)
}

fn parse_wheres(wheres: Vec<(String, String)>) -> Result<Vec<FieldInput>, Error> {
    named_fields(wheres, true, true)
}

fn parse_excludes(excludes: Vec<(String, String)>) -> Result<Vec<FieldInput>, Error> {
    named_fields(excludes, true, false)
}

fn named_fields(
    pairs: Vec<(String, String)>,
    reject_reserved: bool,
    unique: bool,
) -> Result<Vec<FieldInput>, Error> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (name, value) in pairs {
        if reject_reserved && let Some(name) = Identifier::from_reserved(&name) {
            return Err(Error::Usage(Usage::ReservedWhere(name)));
        }
        let name = FieldName::parse(&name)?;
        if unique && !seen.insert(name.clone()) {
            return Err(Error::Usage(Usage::DuplicateField(name)));
        }
        out.push(FieldInput { name, value });
    }
    Ok(out)
}
