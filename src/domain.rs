use std::collections::HashMap;

use crate::db::{Conn, Db, Tx};
use crate::error::{Error, Fail};
use crate::ledger::{
    Agent, Amend, Backup, Entries, FieldInput, FieldValue, Filter, Find, Get, GroupedLink,
    GroupedTime, Ignore, Last, List, Log, NonEmptyFieldValue, Op, Order, Outcome, Posted,
    SchemaAdd, SchemaAddField, SchemaAddValue, SchemaDrop, SchemaRename, SchemaRenameField,
    SchemaRetire, SchemaShow, Schemas, Scope, Stamp, Sum, Summed, Today, Total, Unignore,
};
use crate::mutable_store;
use crate::spec::{EntryId, Field, FieldKind, FieldName, Group, Link, LinkName, SchemaName, Spec};
use crate::store;
use crate::time::{At, Grain, Range};
use jiff::tz::TimeZone;

pub fn execute(db: &mut Db, agent: &Agent, tz: &TimeZone, op: Op) -> Result<Outcome, Error> {
    match op {
        Op::SchemaList => db.read(|conn| {
            Ok(Outcome::Schemas(Schemas {
                schemas: store::list_schemas(conn)?,
            }))
        }),
        Op::SchemaShow(op) => show_schema(db, op),
        Op::SchemaAdd(op) => {
            add_schema(db, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaAddField(op) => {
            add_field(db, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaAddValue(op) => {
            add_value(db, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaRename(op) => {
            rename_schema(db, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaRenameField(op) => {
            rename_field(db, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaRetire(op) => {
            retire(db, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaDrop(op) => {
            drop_schema(db, op)?;
            Ok(Outcome::Empty)
        }
        Op::Log(op) => log(db, agent, op),
        Op::List(op) => list(db, op),
        Op::Get(op) => get(db, op),
        Op::Sum(op) => sum(db, op),
        Op::Last(op) => last(db, op),
        Op::Today(op) => today(db, op, tz),
        Op::Amend(op) => amend(db, op),
        Op::Ignore(op) => ignore(db, op),
        Op::Unignore(op) => unignore(db, op),
        Op::Backup(op) => backup(db, op),
    }
}

fn show_schema(db: &mut Db, op: SchemaShow) -> Result<Outcome, Error> {
    db.read(|conn| Ok(Outcome::Spec(store::load_schema(conn, &op.name)?.spec)))
}

fn add_schema(db: &mut Db, op: SchemaAdd) -> Result<(), Error> {
    db.transaction(|tx| mutable_store::insert_schema(tx, &op.name, &op.spec))
}

fn add_field(db: &mut Db, op: SchemaAddField) -> Result<(), Error> {
    db.transaction(|tx| {
        let loaded = store::load_schema(tx, &op.schema)?;
        if loaded.retired {
            return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
        }
        let field = Field {
            name: op.name.clone(),
            kind: op.kind,
            required: op.default.is_some(),
        };
        if loaded.spec.field(&field.name).is_some() {
            return Err(Error::Fail(Fail::FieldExists(field.name.clone())));
        }
        if let Ok(link_name) = LinkName::parse(field.name.as_str())
            && store::has_outbound_link_name(tx, &op.schema, &link_name)?
        {
            return Err(Error::Fail(Fail::LinkNameCollidesWithField(link_name)));
        }
        mutable_store::add_column(tx, &op.schema, &field, op.default.as_ref())?;
        mutable_store::insert_field(tx, &op.schema, &field, loaded.spec.fields.len() as i64)
    })
}

fn add_value(db: &mut Db, op: SchemaAddValue) -> Result<(), Error> {
    db.transaction(|tx| {
        let loaded = store::load_schema(tx, &op.schema)?;
        if loaded.retired {
            return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
        }
        let Some(f) = loaded.spec.fields.iter().find(|f| f.name == op.field) else {
            return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
        };
        let FieldKind::Enum(values) = &f.kind else {
            return Err(Error::Fail(Fail::FieldNotEnum(op.field.clone())));
        };
        if values.iter().any(|v| v == &op.value) {
            return Err(Error::Fail(Fail::EnumValueExists(op.value)));
        }
        mutable_store::insert_enum_value(tx, &op.schema, &op.field, &op.value, values.len() as i64)
    })
}

fn rename_schema(db: &mut Db, op: SchemaRename) -> Result<(), Error> {
    db.transaction(|tx| mutable_store::rename_schema(tx, &op.from, &op.to))
}

fn rename_field(db: &mut Db, op: SchemaRenameField) -> Result<(), Error> {
    db.transaction(|tx| {
        let loaded = store::load_schema(tx, &op.schema)?;
        if loaded.retired {
            return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
        }
        if loaded.spec.field(&op.from).is_none() {
            return Err(Error::Fail(Fail::UnknownField(op.from.clone())));
        }
        if loaded.spec.field(&op.to).is_some() {
            return Err(Error::Fail(Fail::FieldExists(op.to.clone())));
        }
        if let Ok(link_name) = LinkName::parse(op.to.as_str())
            && store::has_outbound_link_name(tx, &op.schema, &link_name)?
        {
            return Err(Error::Fail(Fail::LinkNameCollidesWithField(link_name)));
        }
        mutable_store::rename_field(tx, &op.schema, &op.from, &op.to)
    })
}

fn retire(db: &mut Db, op: SchemaRetire) -> Result<(), Error> {
    db.transaction(|tx| mutable_store::retire(tx, &op.name))
}

fn drop_schema(db: &mut Db, op: SchemaDrop) -> Result<(), Error> {
    db.transaction(|tx| {
        if store::inbound_link_count(tx, &op.name)? > 0 {
            return Err(Error::Fail(Fail::SchemaHasInboundLinks(op.name.clone())));
        }
        mutable_store::drop_schema(tx, &op.name)
    })
}

fn log(db: &mut Db, agent: &Agent, ops: Vec<Log>) -> Result<Outcome, Error> {
    db.transaction(|tx| {
        let mut out = Vec::new();
        for op in ops {
            out.push(insert_log(tx, agent, op)?);
        }
        Ok(out)
    })
    .map(Outcome::Posted)
}

fn insert_log(tx: &mut Tx<'_>, agent: &Agent, mut op: Log) -> Result<Posted, Error> {
    let agent = op.agent.as_ref().unwrap_or(agent);
    let at = match op.at {
        Some(at) => at,
        None => At::now(),
    };
    let loaded = store::load_schema(tx, &op.schema)?;
    if loaded.retired {
        return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
    }
    let values = given_fields(&loaded.spec, &op.fields)?;
    ensure_required(&loaded.spec, &values)?;
    ensure_links(tx, &loaded.spec, &op.links)?;
    op.links.sort_by(|a, b| a.name.cmp(&b.name));
    let id = mutable_store::insert_entry(
        tx,
        &op.schema,
        &loaded.spec,
        at,
        Some(agent),
        &values,
        &op.links,
    )?;
    Ok(Posted {
        id,
        at,
        links: op.links,
    })
}

fn amend(db: &mut Db, mut op: Amend) -> Result<Outcome, Error> {
    db.transaction(|tx| {
        let loaded = store::load_schema(tx, &op.schema)?;
        if store::get_entry(tx, &op.schema, &loaded.spec, op.id)?.is_none() {
            return Err(Error::Fail(Fail::EntryNotFound {
                schema: op.schema.clone(),
                id: op.id,
            }));
        }
        let values = given_fields(&loaded.spec, &op.fields)?;
        ensure_links(tx, &loaded.spec, &op.links)?;
        mutable_store::update_entry(tx, &op.schema, op.id, op.at, op.agent.as_ref(), &values)?;
        for name in &op.unlinks {
            mutable_store::delete_link(tx, &op.schema, op.id, name)?;
        }
        op.links.sort_by(|a, b| a.name.cmp(&b.name));
        for link in &op.links {
            mutable_store::upsert_link(tx, &op.schema, op.id, link)?;
        }
        let entry = store::get_entry(tx, &op.schema, &loaded.spec, op.id)?.ok_or_else(|| {
            Error::Fail(Fail::EntryNotFound {
                schema: op.schema.clone(),
                id: op.id,
            })
        })?;
        Ok(Outcome::Posted(vec![Posted {
            id: op.id,
            at: entry.at,
            links: entry.links,
        }]))
    })
}

fn ignore(db: &mut Db, op: Ignore) -> Result<Outcome, Error> {
    set_ignored(db, op.schema, op.id, true)
}

fn unignore(db: &mut Db, op: Unignore) -> Result<Outcome, Error> {
    set_ignored(db, op.schema, op.id, false)
}

fn backup(db: &Db, op: Backup) -> Result<Outcome, Error> {
    // VACUUM INTO cannot run inside an open transaction.
    db.backup(&op.path)?;
    Ok(Outcome::Empty)
}

fn set_ignored(
    db: &mut Db,
    schema: SchemaName,
    id: EntryId,
    ignored: bool,
) -> Result<Outcome, Error> {
    db.transaction(|tx| {
        let Some(at) = store::entry_at(tx, &schema, id)? else {
            return Err(Error::Fail(Fail::EntryNotFound {
                schema: schema.clone(),
                id,
            }));
        };
        mutable_store::set_ignored(tx, &schema, id, ignored)?;
        Ok(Outcome::Stamp(Stamp { id, at }))
    })
}

fn get(db: &mut Db, op: Get) -> Result<Outcome, Error> {
    db.read(|conn| {
        let spec = store::load_schema(conn, &op.schema)?.spec;
        let Some(entry) = store::get_entry(conn, &op.schema, &spec, op.id)? else {
            return Err(Error::Fail(Fail::EntryNotFound {
                schema: op.schema.clone(),
                id: op.id,
            }));
        };
        Ok(Outcome::Entries(Entries {
            spec,
            entries: vec![entry],
            include_ignored: true,
        }))
    })
}

fn last(db: &mut Db, op: Last) -> Result<Outcome, Error> {
    let outcome = find_entries(
        db,
        Query {
            scope: &op.scope,
            range: Range::default(),
            include_ignored: false,
            order: Order::Newest,
            limit: Some(1),
            max_grain: None,
        },
    )?;
    match &outcome {
        Outcome::Entries(Entries { entries, .. }) if entries.is_empty() => {
            Err(Error::Fail(Fail::NotFound))
        }
        _ => Ok(outcome),
    }
}

fn today(db: &mut Db, op: Today, tz: &TimeZone) -> Result<Outcome, Error> {
    find_entries(
        db,
        Query {
            scope: &op.scope,
            range: Range::today(tz)?,
            include_ignored: false,
            order: Order::Oldest,
            limit: None,
            max_grain: Some(Grain::Day),
        },
    )
}

fn list(db: &mut Db, op: List) -> Result<Outcome, Error> {
    find_entries(
        db,
        Query {
            scope: &op.scope,
            range: op.range,
            include_ignored: op.include_ignored,
            order: Order::Oldest,
            limit: None,
            max_grain: None,
        },
    )
}

struct Query<'a> {
    scope: &'a Scope,
    range: Range,
    include_ignored: bool,
    order: Order,
    limit: Option<usize>,
    max_grain: Option<Grain>,
}

fn find_entries(db: &mut Db, q: Query<'_>) -> Result<Outcome, Error> {
    db.read(|conn| {
        let spec = store::load_schema(conn, &q.scope.schema)?.spec;
        let resolved = resolve_filters(&spec, &q.scope.fields, &q.scope.links)?;
        let excludes = resolve_filters(&spec, &q.scope.excludes, &[])?;
        let entries = store::find(
            conn,
            Find {
                schema: &q.scope.schema,
                spec: &spec,
                range: q.range,
                agent: q.scope.agent.as_ref(),
                include_ignored: q.include_ignored,
                filters: &resolved,
                excludes: &excludes,
                order: q.order,
                limit: q.limit,
                max_grain: q.max_grain,
            },
        )?;
        Ok(Outcome::Entries(Entries {
            spec,
            entries,
            include_ignored: q.include_ignored,
        }))
    })
}

fn sum(db: &mut Db, op: Sum) -> Result<Outcome, Error> {
    db.read(|conn| {
        let spec = store::load_schema(conn, &op.scope.schema)?.spec;
        let Some(f) = spec.field(&op.field) else {
            return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
        };
        if !matches!(f.kind, FieldKind::Number) {
            return Err(Error::Fail(Fail::FieldNotNumber(op.field.clone())));
        }
        let resolved = resolve_filters(&spec, &op.scope.fields, &op.scope.links)?;
        let excludes = resolve_filters(&spec, &op.scope.excludes, &[])?;
        let q = Find {
            schema: &op.scope.schema,
            spec: &spec,
            range: op.range,
            agent: op.scope.agent.as_ref(),
            include_ignored: false,
            filters: &resolved,
            excludes: &excludes,
            order: Order::Oldest,
            limit: None,
            max_grain: match op.group {
                Some(Group::Time(unit)) => Some(Grain::for_group(unit)),
                _ => None,
            },
        };
        if let Some(Group::Link(name)) = &op.group {
            spec.ensure_link_name(name)?;
        }
        Ok(match store::sum(conn, q, &op.field, op.group)? {
            Summed::Total(value) => Outcome::Total(Total {
                field: op.field,
                value,
            }),
            Summed::Time { unit, buckets } => Outcome::GroupedTime(GroupedTime { unit, buckets }),
            Summed::Link { name, buckets } => Outcome::GroupedLink(GroupedLink { name, buckets }),
        })
    })
}

fn resolve_filters(
    spec: &Spec,
    fields: &[FieldInput],
    links: &[Link],
) -> Result<Vec<Filter>, Error> {
    let mut out = Vec::new();
    for field in fields {
        let Some(spec_field) = spec.field(&field.name) else {
            return Err(Error::Fail(Fail::UnknownField(field.name.clone())));
        };
        out.push(Filter::Field {
            name: spec_field.name.clone(),
            value: NonEmptyFieldValue::parse(spec_field, &field.value)?,
        });
    }
    for link in links {
        spec.ensure_link_name(&link.name)?;
        out.push(Filter::Link {
            name: link.name.clone(),
            to: link.to.clone(),
        });
    }
    Ok(out)
}

fn ensure_links(conn: &impl Conn, spec: &Spec, links: &[Link]) -> Result<(), Error> {
    for link in links {
        spec.ensure_link_name(&link.name)?;
        if !store::entry_exists(conn, &link.to.schema, link.to.id)? {
            return Err(Error::Fail(Fail::LinkTargetMissing(link.to.clone())));
        }
    }
    Ok(())
}

fn given_fields(
    spec: &Spec,
    fields: &[FieldInput],
) -> Result<HashMap<FieldName, FieldValue>, Error> {
    let mut out = HashMap::new();
    for field in fields {
        let Some(spec_field) = spec.field(&field.name) else {
            return Err(Error::Fail(Fail::UnknownField(field.name.clone())));
        };
        let value = FieldValue::parse(spec_field, &field.value)?;
        if spec_field.required && matches!(value, FieldValue::Empty) {
            return Err(Error::Fail(Fail::MissingRequiredField(field.name.clone())));
        }
        out.insert(field.name.clone(), value);
    }
    Ok(out)
}

fn ensure_required(spec: &Spec, values: &HashMap<FieldName, FieldValue>) -> Result<(), Error> {
    for spec_field in &spec.fields {
        if spec_field.required && !values.contains_key(&spec_field.name) {
            return Err(Error::Fail(Fail::MissingRequiredField(
                spec_field.name.clone(),
            )));
        }
    }
    Ok(())
}
