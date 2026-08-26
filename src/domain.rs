use std::collections::HashMap;

use crate::db::{Connection, Db};
use crate::error::{Error, Fail};
use crate::ledger::{
    Agent, Amend, Entries, FieldInput, FieldValue, Filter, Get, GroupedLink, GroupedTime, Ignore,
    Last, List, Log, Op, Order, Outcome, Posted, SchemaAdd, SchemaAddField, SchemaAddValue,
    SchemaDrop, SchemaRetire, SchemaShow, Schemas, Stamp, Sum, Today, Total,
};
use crate::mutable_store;
use crate::spec::{FieldKind, FieldName, Group, Link, LinkName, SchemaName, Spec};
use crate::store::{self, Find};
use crate::time::{Instant, Range};
use jiff::tz::TimeZone;

pub fn execute(db: &mut Db, agent: &Agent, tz: &TimeZone, op: Op) -> Result<Outcome, Error> {
    match op {
        Op::SchemaList => Ok(Outcome::Schemas(Schemas {
            schemas: store::list_schemas(db)?,
        })),
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
        Op::Sum(op) => sum(db, op, tz),
        Op::Last(op) => last(db, op),
        Op::Today(op) => today(db, op, tz),
        Op::Amend(op) => amend(db, op),
        Op::Ignore(op) => ignore(db, op),
    }
}

fn show_schema(db: &Db, op: SchemaShow) -> Result<Outcome, Error> {
    Ok(Outcome::Spec(store::load_schema(db, &op.name)?.spec))
}

fn add_schema(db: &mut Db, op: SchemaAdd) -> Result<(), Error> {
    db.transaction(|tx| mutable_store::insert_schema(tx, &op.name, &op.spec))
}

fn add_field(db: &mut Db, op: SchemaAddField) -> Result<(), Error> {
    db.transaction(|tx| {
        let mut kind = store::load_schema(tx, &op.schema)?;
        if kind.retired {
            return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
        }
        if kind.spec.field(&op.field.name).is_some() {
            return Err(Error::Fail(Fail::FieldExists(op.field.name.clone())));
        }
        mutable_store::add_column(tx, &op.schema, &op.field, op.default.as_ref())?;
        kind.spec.fields.push(op.field);
        mutable_store::save_spec(tx, &op.schema, &kind.spec)
    })
}

fn add_value(db: &mut Db, op: SchemaAddValue) -> Result<(), Error> {
    db.transaction(|tx| {
        let mut kind = store::load_schema(tx, &op.schema)?;
        if kind.retired {
            return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
        }
        let Some(f) = kind.spec.fields.iter_mut().find(|f| f.name == op.field) else {
            return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
        };
        let FieldKind::Enum(values) = &mut f.kind else {
            return Err(Error::Fail(Fail::FieldNotEnum(op.field.clone())));
        };
        if values.iter().any(|v| v == &op.value) {
            return Err(Error::Fail(Fail::EnumValueExists(op.value)));
        }
        values.push(op.value);
        mutable_store::save_spec(tx, &op.schema, &kind.spec)
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

fn log(db: &mut Db, agent: &Agent, mut op: Log) -> Result<Outcome, Error> {
    let agent = op.agent.as_ref().unwrap_or(agent);
    let at = op.at.unwrap_or_else(Instant::now);
    let id = db.transaction(|tx| {
        let kind = store::load_schema(tx, &op.schema)?;
        if kind.retired {
            return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
        }
        let values = prepare_fields(&kind.spec, &op.fields, false)?;
        ensure_links(tx, &kind.spec, &op.links)?;
        op.links.sort_by(|a, b| a.name.cmp(&b.name));
        mutable_store::insert_entry(
            tx,
            &op.schema,
            &kind.spec,
            at,
            Some(agent.as_str()),
            &values,
            &op.links,
        )
    })?;
    Ok(Outcome::Posted(Posted {
        id,
        at,
        links: op.links,
    }))
}

fn amend(db: &mut Db, mut op: Amend) -> Result<Outcome, Error> {
    db.transaction(|tx| {
        let kind = store::load_schema(tx, &op.schema)?;
        if store::get_entry(tx, &op.schema, &kind.spec, op.id)?.is_none() {
            return Err(Error::Fail(Fail::EntryNotFound {
                schema: op.schema.clone(),
                id: op.id,
            }));
        }
        let values = prepare_fields(&kind.spec, &op.fields, true)?;
        ensure_links(tx, &kind.spec, &op.links)?;
        mutable_store::update_entry(
            tx,
            &op.schema,
            op.id,
            op.at,
            op.agent.as_ref().map(Agent::as_str),
            &values,
        )?;
        for name in &op.unlinks {
            mutable_store::delete_link(tx, &op.schema, op.id, name)?;
        }
        op.links.sort_by(|a, b| a.name.cmp(&b.name));
        for link in &op.links {
            mutable_store::upsert_link(tx, &op.schema, op.id, link)?;
        }
        let entry = store::get_entry(tx, &op.schema, &kind.spec, op.id)?.ok_or_else(|| {
            Error::Fail(Fail::EntryNotFound {
                schema: op.schema.clone(),
                id: op.id,
            })
        })?;
        Ok(Outcome::Posted(Posted {
            id: op.id,
            at: entry.at,
            links: entry.links,
        }))
    })
}

fn ignore(db: &mut Db, op: Ignore) -> Result<Outcome, Error> {
    db.transaction(|tx| {
        let spec = store::load_schema(tx, &op.schema)?.spec;
        let Some(entry) = store::get_entry(tx, &op.schema, &spec, op.id)? else {
            return Err(Error::Fail(Fail::EntryNotFound {
                schema: op.schema.clone(),
                id: op.id,
            }));
        };
        mutable_store::set_ignored(tx, &op.schema, op.id)?;
        Ok(Outcome::Stamp(Stamp {
            id: op.id,
            at: entry.at,
        }))
    })
}

fn get(db: &Db, op: Get) -> Result<Outcome, Error> {
    let spec = store::load_schema(db, &op.schema)?.spec;
    let Some(entry) = store::get_entry(db, &op.schema, &spec, op.id)? else {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: op.schema.clone(),
            id: op.id,
        }));
    };
    Ok(Outcome::Entries(Entries {
        spec,
        entries: vec![entry],
    }))
}

fn last(db: &Db, op: Last) -> Result<Outcome, Error> {
    let outcome = find_entries(
        db,
        Query {
            schema: &op.schema,
            range: Range::default(),
            agent: op.agent.as_ref().map(Agent::as_str),
            fields: &op.fields,
            links: &op.links,
            include_ignored: false,
            order: Order::Newest,
            limit: Some(1),
        },
    )?;
    match &outcome {
        Outcome::Entries(Entries { entries, .. }) if entries.is_empty() => {
            Err(Error::Fail(Fail::NotFound))
        }
        _ => Ok(outcome),
    }
}

fn today(db: &Db, op: Today, tz: &TimeZone) -> Result<Outcome, Error> {
    find_entries(
        db,
        Query {
            schema: &op.schema,
            range: Range::today(tz)?,
            agent: op.agent.as_ref().map(Agent::as_str),
            fields: &op.fields,
            links: &op.links,
            include_ignored: false,
            order: Order::Oldest,
            limit: None,
        },
    )
}

fn list(db: &Db, op: List) -> Result<Outcome, Error> {
    find_entries(
        db,
        Query {
            schema: &op.schema,
            range: op.range,
            agent: op.agent.as_ref().map(Agent::as_str),
            fields: &op.fields,
            links: &op.links,
            include_ignored: op.include_ignored,
            order: Order::Oldest,
            limit: None,
        },
    )
}

struct Query<'a> {
    schema: &'a SchemaName,
    range: Range,
    agent: Option<&'a str>,
    fields: &'a [FieldInput],
    links: &'a [Link],
    include_ignored: bool,
    order: Order,
    limit: Option<usize>,
}

fn find_entries(db: &Db, q: Query<'_>) -> Result<Outcome, Error> {
    let spec = store::load_schema(db, q.schema)?.spec;
    let resolved = resolve_filters(&spec, q.fields, q.links)?;
    let entries = store::find(
        db,
        Find {
            schema: q.schema,
            spec: &spec,
            range: q.range,
            agent: q.agent,
            include_ignored: q.include_ignored,
            filters: &resolved,
            order: q.order,
            limit: q.limit,
        },
    )?;
    Ok(Outcome::Entries(Entries { spec, entries }))
}

fn sum(db: &Db, op: Sum, tz: &TimeZone) -> Result<Outcome, Error> {
    let spec = store::load_schema(db, &op.schema)?.spec;
    let Some(f) = spec.field(&op.field) else {
        return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
    };
    if !matches!(f.kind, FieldKind::Number) {
        return Err(Error::Fail(Fail::FieldNotNumber(op.field.clone())));
    }
    let resolved = resolve_filters(&spec, &op.fields, &op.links)?;
    let q = Find {
        schema: &op.schema,
        spec: &spec,
        range: op.range,
        agent: op.agent.as_ref().map(Agent::as_str),
        include_ignored: false,
        filters: &resolved,
        order: Order::Oldest,
        limit: None,
    };
    if let Some(Group::Link(name)) = &op.group
        && field_named(&spec, name)
    {
        return Err(Error::Fail(Fail::LinkNameCollidesWithField(name.clone())));
    }
    Ok(match store::sum(db, q, &op.field, op.group, tz)? {
        store::Summed::Total(value) => Outcome::Total(Total {
            field: op.field,
            value,
        }),
        store::Summed::Time { unit, buckets } => {
            Outcome::GroupedTime(GroupedTime { unit, buckets })
        }
        store::Summed::Link { name, buckets } => {
            Outcome::GroupedLink(GroupedLink { name, buckets })
        }
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
            value: FieldValue::parse(spec_field, &field.value)?,
        });
    }
    for link in links {
        if field_named(spec, &link.name) {
            return Err(Error::Fail(Fail::LinkNameCollidesWithField(
                link.name.clone(),
            )));
        }
        out.push(Filter::Link {
            name: link.name.clone(),
            to: link.to.clone(),
        });
    }
    Ok(out)
}

fn field_named(spec: &Spec, name: &LinkName) -> bool {
    FieldName::parse(name.as_str())
        .ok()
        .is_some_and(|n| spec.field(&n).is_some())
}

fn ensure_links(conn: &impl Connection, spec: &Spec, links: &[Link]) -> Result<(), Error> {
    for link in links {
        if field_named(spec, &link.name) {
            return Err(Error::Fail(Fail::LinkNameCollidesWithField(
                link.name.clone(),
            )));
        }
        let target = store::load_schema(conn, &link.to.schema)?;
        if store::get_entry(conn, &link.to.schema, &target.spec, link.to.id)?.is_none() {
            return Err(Error::Fail(Fail::LinkTargetMissing(link.to.clone())));
        }
    }
    Ok(())
}

fn prepare_fields(
    spec: &Spec,
    fields: &[FieldInput],
    partial: bool,
) -> Result<HashMap<FieldName, FieldValue>, Error> {
    let mut out = HashMap::new();
    for field in fields {
        let Some(spec_field) = spec.field(&field.name) else {
            return Err(Error::Fail(Fail::UnknownField(field.name.clone())));
        };
        if field.value.is_empty() {
            if spec_field.required {
                return Err(Error::Fail(Fail::MissingRequiredField(field.name.clone())));
            }
            out.insert(field.name.clone(), FieldValue::Empty);
            continue;
        }
        out.insert(
            field.name.clone(),
            FieldValue::parse(spec_field, &field.value)?,
        );
    }
    if !partial {
        for spec_field in &spec.fields {
            if spec_field.required && !out.contains_key(&spec_field.name) {
                return Err(Error::Fail(Fail::MissingRequiredField(
                    spec_field.name.clone(),
                )));
            }
        }
    }
    Ok(out)
}
