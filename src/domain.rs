use std::collections::{BTreeMap, HashMap, HashSet};

use rust_decimal::Decimal;

use crate::db::Db;
use crate::error::{Error, Fail, Usage};
use crate::ledger::{
    Agent, Amend, Clause, Entries, Entry, FieldInput, FieldValue, Filter, Get, GroupedLink,
    GroupedTime, Ignore, Last, List, Log, Op, Order, Outcome, Posted, SchemaAdd, SchemaAddField,
    SchemaAddValue, SchemaDrop, SchemaRetire, SchemaShow, Schemas, Stamp, Sum, Today, Total,
};
use crate::mutable_store;
use crate::spec::{
    EntryRef, Field, FieldName, FieldType, Group, Link, LinkName, SchemaName, Spec, fold_enum,
    fold_enum_values, parse_number,
};
use crate::store::{self, Find};
use crate::time::{self, Instant, Period, Range};

pub(crate) fn execute(db: &mut Db, agent: &Agent, op: Op) -> Result<Outcome, Error> {
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
        Op::Sum(op) => sum(db, op),
        Op::Last(op) => last(db, op),
        Op::Today(op) => today(db, op),
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
    let mut kind = store::load_schema(db, &op.schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
    }
    if kind.spec.field(&op.name).is_some() {
        return Err(Error::Fail(Fail::FieldExists(op.name.clone())));
    }
    let values = if op.type_ == FieldType::Enum {
        let Some(vals) = op.values else {
            return Err(Error::Usage(Usage::EnumValuesRequired));
        };
        Some(fold_enum_values(vals)?)
    } else if op.values.is_some() {
        return Err(Error::Usage(Usage::EnumValuesNotAllowed));
    } else {
        None
    };
    let required = op.default.is_some();
    if let Some(ref def) = op.default {
        parse_field_value_parts(&op.name, op.type_, values.as_deref(), def)?;
    }
    let field = Field {
        name: op.name,
        type_: op.type_,
        required,
        values,
    };
    db.transaction(|tx| {
        mutable_store::add_column(tx, &op.schema, &field, op.default.as_deref())?;
        kind.spec.fields.push(field);
        mutable_store::save_spec(tx, &op.schema, &kind.spec)
    })
}

fn add_value(db: &mut Db, op: SchemaAddValue) -> Result<(), Error> {
    let mut kind = store::load_schema(db, &op.schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
    }
    let Some(f) = kind.spec.fields.iter_mut().find(|f| f.name == op.field) else {
        return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
    };
    if f.type_ != FieldType::Enum {
        return Err(Error::Fail(Fail::FieldNotEnum(op.field.clone())));
    }
    let folded = fold_enum(&op.value)?;
    let values = f.values.get_or_insert_with(Vec::new);
    if values.iter().any(|v| v == &folded) {
        return Err(Error::Fail(Fail::EnumValueExists(folded)));
    }
    values.push(folded);
    db.transaction(|tx| mutable_store::save_spec(tx, &op.schema, &kind.spec))
}

fn retire(db: &mut Db, op: SchemaRetire) -> Result<(), Error> {
    db.transaction(|tx| {
        if mutable_store::retire(tx, &op.name)? == 0 {
            return Err(Error::Fail(Fail::UnknownSchema(op.name.clone())));
        }
        Ok(())
    })
}

fn drop_schema(db: &mut Db, op: SchemaDrop) -> Result<(), Error> {
    db.transaction(|tx| {
        if !mutable_store::schema_exists(tx, &op.name)? {
            return Err(Error::Fail(Fail::UnknownSchema(op.name.clone())));
        }
        if mutable_store::inbound_link_count(tx, &op.name)? > 0 {
            return Err(Error::Fail(Fail::SchemaHasInboundLinks(op.name.clone())));
        }
        mutable_store::drop_schema(tx, &op.name)
    })
}

fn log(db: &mut Db, agent: &Agent, mut op: Log) -> Result<Outcome, Error> {
    let kind = store::load_schema(db, &op.schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
    }
    let agent = op.agent.as_ref().unwrap_or(agent);
    let at = op.at.unwrap_or_else(Instant::now);
    let values = prepare_fields(&kind.spec, &op.fields, false)?;
    ensure_links(db, &kind.spec, &op.links)?;
    op.links.sort_by(|a, b| a.name.cmp(&b.name));
    let id = db.transaction(|tx| {
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
    if op.at.is_none()
        && op.agent.is_none()
        && op.links.is_empty()
        && op.unlinks.is_empty()
        && op.fields.is_empty()
    {
        return Err(Error::Usage(Usage::AmendEmpty));
    }
    let kind = store::load_schema(db, &op.schema)?;
    if store::get_entry(db, &op.schema, &kind.spec, op.id)?.is_none() {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: op.schema.clone(),
            id: op.id,
        }));
    }
    let mut unlink_set = HashSet::new();
    for name in &op.unlinks {
        if !unlink_set.insert(name) {
            return Err(Error::Usage(Usage::DuplicateUnlink(name.clone())));
        }
        if op.links.iter().any(|l| &l.name == name) {
            return Err(Error::Usage(Usage::LinkAndUnlink(name.clone())));
        }
    }
    let values = prepare_fields(&kind.spec, &op.fields, true)?;
    ensure_links(db, &kind.spec, &op.links)?;
    db.transaction(|tx| {
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
        let entry =
            mutable_store::get_entry(tx, &op.schema, &kind.spec, op.id)?.ok_or_else(|| {
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
    let spec = store::load_schema(db, &op.schema)?.spec;
    let Some(entry) = store::get_entry(db, &op.schema, &spec, op.id)? else {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: op.schema.clone(),
            id: op.id,
        }));
    };
    db.transaction(|tx| mutable_store::set_ignored(tx, &op.schema, op.id))?;
    Ok(Outcome::Stamp(Stamp {
        id: op.id,
        at: entry.at,
    }))
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
            filters: &op.filters,
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

fn today(db: &Db, op: Today) -> Result<Outcome, Error> {
    find_entries(
        db,
        Query {
            schema: &op.schema,
            range: Range::today()?,
            agent: op.agent.as_ref().map(Agent::as_str),
            filters: &op.filters,
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
            filters: &op.filters,
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
    filters: &'a [Clause],
    include_ignored: bool,
    order: Order,
    limit: Option<usize>,
}

fn find_entries(db: &Db, q: Query<'_>) -> Result<Outcome, Error> {
    let spec = store::load_schema(db, q.schema)?.spec;
    let resolved = resolve_filters(&spec, q.filters)?;
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

fn sum(db: &Db, op: Sum) -> Result<Outcome, Error> {
    let spec = store::load_schema(db, &op.schema)?.spec;
    let Some(f) = spec.field(&op.field) else {
        return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
    };
    if f.type_ != FieldType::Number {
        return Err(Error::Fail(Fail::FieldNotNumber(op.field.clone())));
    }
    let resolved = resolve_filters(&spec, &op.filters)?;
    let entries = store::find(
        db,
        Find {
            schema: &op.schema,
            spec: &spec,
            range: op.range,
            agent: op.agent.as_ref().map(Agent::as_str),
            include_ignored: false,
            filters: &resolved,
            order: Order::Oldest,
            limit: None,
        },
    )?;
    match op.group {
        None => {
            let total: Decimal = entries.iter().filter_map(|e| e.number(&op.field)).sum();
            Ok(Outcome::Total(Total {
                field: op.field,
                value: total,
            }))
        }
        Some(Group::Time(unit)) => grouped_time(&entries, &op.field, unit),
        Some(Group::Link(name)) => {
            if field_named(&spec, &name) {
                return Err(Error::Fail(Fail::LinkNameCollidesWithField(name)));
            }
            grouped_link(&entries, &op.field, name)
        }
    }
}

fn grouped_time(
    entries: &[Entry],
    field: &FieldName,
    unit: crate::spec::TimePeriod,
) -> Result<Outcome, Error> {
    let mut buckets: BTreeMap<Period, Decimal> = BTreeMap::new();
    for entry in entries {
        let k = time::period(unit, entry.at);
        *buckets.entry(k).or_insert(Decimal::ZERO) += entry.number(field).unwrap_or(Decimal::ZERO);
    }
    Ok(Outcome::GroupedTime(GroupedTime {
        unit,
        buckets: buckets.into_iter().collect(),
    }))
}

fn grouped_link(entries: &[Entry], field: &FieldName, name: LinkName) -> Result<Outcome, Error> {
    let mut buckets: BTreeMap<Option<EntryRef>, Decimal> = BTreeMap::new();
    for entry in entries {
        let key = entry
            .links
            .iter()
            .find(|l| l.name == name)
            .map(|l| l.to.clone());
        *buckets.entry(key).or_insert(Decimal::ZERO) +=
            entry.number(field).unwrap_or(Decimal::ZERO);
    }
    Ok(Outcome::GroupedLink(GroupedLink {
        name,
        buckets: buckets.into_iter().collect(),
    }))
}

fn resolve_filters(spec: &Spec, filters: &[Clause]) -> Result<Vec<Filter>, Error> {
    let mut out = Vec::new();
    for clause in filters {
        if let Ok(name) = FieldName::parse(clause.name.as_str())
            && let Some(field) = spec.field(&name)
        {
            let value = match field.type_ {
                FieldType::Number => FieldValue::Number(parse_number(&clause.value)?),
                FieldType::Enum => FieldValue::Enum(fold_enum(&clause.value)?),
                FieldType::Text => FieldValue::Text(clause.value.clone()),
            };
            out.push(Filter::Field {
                name: field.name.clone(),
                value,
            });
            continue;
        }
        let link_name = LinkName::parse(clause.name.as_str())?;
        let to = EntryRef::parse(&clause.value)?;
        out.push(Filter::Link {
            name: link_name,
            to,
        });
    }
    Ok(out)
}

fn field_named(spec: &Spec, name: &LinkName) -> bool {
    FieldName::parse(name.as_str())
        .ok()
        .is_some_and(|n| spec.field(&n).is_some())
}

fn ensure_links(db: &Db, spec: &Spec, links: &[Link]) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for link in links {
        if !seen.insert(&link.name) {
            return Err(Error::Usage(Usage::DuplicateLinkName(link.name.clone())));
        }
        if field_named(spec, &link.name) {
            return Err(Error::Fail(Fail::LinkNameCollidesWithField(
                link.name.clone(),
            )));
        }
        let target = store::load_schema(db, &link.to.schema)?;
        if store::get_entry(db, &link.to.schema, &target.spec, link.to.id)?.is_none() {
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
    let mut seen = HashSet::new();
    let mut out = HashMap::new();
    for field in fields {
        if !seen.insert(&field.name) {
            return Err(Error::Usage(Usage::DuplicateField(field.name.clone())));
        }
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
            parse_field_value(spec_field, &field.value)?,
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

fn parse_field_value(field: &Field, value: &str) -> Result<FieldValue, Error> {
    parse_field_value_parts(&field.name, field.type_, field.values.as_deref(), value)
}

fn parse_field_value_parts(
    name: &FieldName,
    type_: FieldType,
    values: Option<&[crate::spec::EnumValue]>,
    value: &str,
) -> Result<FieldValue, Error> {
    match type_ {
        FieldType::Text => {
            if value.contains('\t') || value.contains('\n') {
                return Err(Error::Fail(Fail::TextHasTabOrNewline(name.clone())));
            }
            Ok(FieldValue::Text(value.to_string()))
        }
        FieldType::Number => Ok(FieldValue::Number(parse_number(value)?)),
        FieldType::Enum => {
            let folded = fold_enum(value)?;
            let Some(values) = values else {
                return Err(Error::Fail(Fail::EnumHasNoValues(name.clone())));
            };
            if !values.iter().any(|v| v == &folded) {
                return Err(Error::Fail(Fail::InvalidEnumValue {
                    field: name.clone(),
                    value: value.to_string(),
                }));
            }
            Ok(FieldValue::Enum(folded))
        }
    }
}
