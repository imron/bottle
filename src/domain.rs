use std::collections::{BTreeMap, HashMap, HashSet};

use rust_decimal::Decimal;

use crate::error::{Error, Fail, Usage};
use crate::ledger::{
    Amend, Entry, FieldValue, Filter, Get, Ignore, Last, List, Log, Op, Order, Outcome, SchemaAdd,
    SchemaAddField, SchemaAddValue, SchemaDrop, SchemaRetire, SchemaShow, Sum, Today,
};
use crate::mutable_store::Tx;
use crate::spec::{
    EntryRef, Field, FieldName, FieldType, Group, Link, LinkName, SchemaName, Spec, fold_enum,
    fold_enum_values, is_reserved, parse_number,
};
use crate::store::{Find, Store};
use crate::time::{self, Instant, Range};

pub(crate) fn execute(
    store: &mut Store,
    default_agent: Option<&str>,
    op: Op,
) -> Result<Outcome, Error> {
    match op {
        Op::SchemaList => Ok(Outcome::Schemas(store.list_schemas()?)),
        Op::SchemaShow(op) => show_schema(store, op),
        Op::SchemaAdd(op) => {
            add_schema(store, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaAddField(op) => {
            add_field(store, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaAddValue(op) => {
            add_value(store, op)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaRetire(op) => store.transaction(|tx| {
            retire(tx, &op)?;
            Ok(Outcome::Empty)
        }),
        Op::SchemaDrop(op) => store.transaction(|tx| {
            drop_schema(tx, &op)?;
            Ok(Outcome::Empty)
        }),
        Op::Log(op) => log(store, default_agent, op),
        Op::List(op) => list(store, op),
        Op::Get(op) => get(store, op),
        Op::Sum(op) => sum(store, op),
        Op::Last(op) => last(store, op),
        Op::Today(op) => today(store, op),
        Op::Amend(op) => amend(store, op),
        Op::Ignore(op) => ignore(store, op),
    }
}

fn show_schema(store: &Store, op: SchemaShow) -> Result<Outcome, Error> {
    Ok(Outcome::Spec(store.load_schema(&op.name)?.spec))
}

fn add_schema(store: &mut Store, op: SchemaAdd) -> Result<(), Error> {
    store.transaction(|tx| tx.insert_schema(&op.name, &op.spec))
}

fn add_field(store: &mut Store, mut op: SchemaAddField) -> Result<(), Error> {
    let mut kind = store.load_schema(&op.schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
    }
    if kind.spec.field(op.name.as_str()).is_some() {
        return Err(Error::Fail(Fail::FieldExists(op.name.clone())));
    }
    if op.type_ == FieldType::Enum {
        let Some(vals) = op.values.as_mut() else {
            return Err(Error::Usage(Usage::EnumValuesRequired));
        };
        fold_enum_values(vals)?;
    } else if op.values.is_some() {
        return Err(Error::Usage(Usage::EnumValuesNotAllowed));
    }
    let required = op.default.is_some();
    if let Some(ref def) = op.default {
        parse_field_value_parts(op.name.as_str(), op.type_, op.values.as_deref(), def)?;
    }
    let field = Field {
        name: op.name.as_str().to_string(),
        type_: op.type_,
        required,
        values: op.values,
    };
    store.transaction(|tx| {
        tx.add_column(&op.schema, &field, op.default.as_deref())?;
        kind.spec.fields.push(field);
        tx.save_spec(&op.schema, &kind.spec)
    })
}

fn add_value(store: &mut Store, op: SchemaAddValue) -> Result<(), Error> {
    let mut kind = store.load_schema(&op.schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
    }
    let Some(f) = kind
        .spec
        .fields
        .iter_mut()
        .find(|f| f.name == op.field.as_str())
    else {
        return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
    };
    if f.type_ != FieldType::Enum {
        return Err(Error::Fail(Fail::FieldNotEnum(op.field.clone())));
    }
    let folded = fold_enum(&op.value);
    let values = f.values.get_or_insert_with(Vec::new);
    if values.iter().any(|v| v == &folded) {
        return Err(Error::Fail(Fail::EnumValueExists(folded)));
    }
    values.push(folded);
    store.transaction(|tx| tx.save_spec(&op.schema, &kind.spec))
}

fn retire(tx: &mut Tx<'_>, op: &SchemaRetire) -> Result<(), Error> {
    if tx.retire(&op.name)? == 0 {
        return Err(Error::Fail(Fail::UnknownSchema(op.name.clone())));
    }
    Ok(())
}

fn drop_schema(tx: &mut Tx<'_>, op: &SchemaDrop) -> Result<(), Error> {
    if !tx.schema_exists(&op.name)? {
        return Err(Error::Fail(Fail::UnknownSchema(op.name.clone())));
    }
    if tx.inbound_link_count(&op.name)? > 0 {
        return Err(Error::Fail(Fail::SchemaHasInboundLinks(op.name.clone())));
    }
    tx.drop_schema(&op.name)
}

fn log(store: &mut Store, default_agent: Option<&str>, mut op: Log) -> Result<Outcome, Error> {
    let kind = store.load_schema(&op.schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(op.schema.clone())));
    }
    let agent = op
        .agent
        .or_else(|| default_agent.map(str::to_string))
        .or_else(|| Some("bottle".to_string()));
    let at = op.at.unwrap_or_else(Instant::now);
    let values = prepare_fields(&kind.spec, &op.fields, false)?;
    ensure_links(store, &kind.spec, &op.links)?;
    op.links
        .sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    let id = store.transaction(|tx| {
        tx.insert_entry(
            &op.schema,
            &kind.spec,
            at,
            agent.as_deref(),
            &values,
            &op.links,
        )
    })?;
    Ok(Outcome::Posted {
        id,
        at,
        links: op.links,
    })
}

fn amend(store: &mut Store, mut op: Amend) -> Result<Outcome, Error> {
    if op.at.is_none()
        && op.agent.is_none()
        && op.links.is_empty()
        && op.unlinks.is_empty()
        && op.fields.is_empty()
    {
        return Err(Error::Usage(Usage::AmendEmpty));
    }
    let kind = store.load_schema(&op.schema)?;
    if store.get_entry(&op.schema, &kind.spec, op.id)?.is_none() {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: op.schema.clone(),
            id: op.id,
        }));
    }
    let mut unlink_set = HashSet::new();
    for name in &op.unlinks {
        if !unlink_set.insert(name.as_str()) {
            return Err(Error::Usage(Usage::DuplicateUnlink(name.clone())));
        }
        if op.links.iter().any(|l| l.name.as_str() == name.as_str()) {
            return Err(Error::Usage(Usage::LinkAndUnlink(name.clone())));
        }
    }
    let values = prepare_fields(&kind.spec, &op.fields, true)?;
    ensure_links(store, &kind.spec, &op.links)?;
    store.transaction(|tx| {
        tx.update_entry(&op.schema, op.id, op.at, op.agent.as_deref(), &values)?;
        for name in &op.unlinks {
            tx.delete_link(&op.schema, op.id, name)?;
        }
        op.links
            .sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        for link in &op.links {
            tx.upsert_link(&op.schema, op.id, link)?;
        }
        let entry = tx
            .get_entry(&op.schema, &kind.spec, op.id)?
            .ok_or_else(|| {
                Error::Fail(Fail::EntryNotFound {
                    schema: op.schema.clone(),
                    id: op.id,
                })
            })?;
        Ok(Outcome::Posted {
            id: op.id,
            at: entry.at,
            links: entry.links,
        })
    })
}

fn ignore(store: &mut Store, op: Ignore) -> Result<Outcome, Error> {
    let spec = store.load_schema(&op.schema)?.spec;
    let Some(entry) = store.get_entry(&op.schema, &spec, op.id)? else {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: op.schema.clone(),
            id: op.id,
        }));
    };
    store.transaction(|tx| tx.set_ignored(&op.schema, op.id))?;
    Ok(Outcome::Stamp {
        id: op.id,
        at: entry.at,
    })
}

fn get(store: &Store, op: Get) -> Result<Outcome, Error> {
    let spec = store.load_schema(&op.schema)?.spec;
    let Some(entry) = store.get_entry(&op.schema, &spec, op.id)? else {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: op.schema.clone(),
            id: op.id,
        }));
    };
    Ok(Outcome::Entries {
        spec,
        entries: vec![entry],
    })
}

fn last(store: &Store, op: Last) -> Result<Outcome, Error> {
    let outcome = find_entries(
        store,
        Query {
            schema: &op.schema,
            range: Range::default(),
            agent: op.agent.as_deref(),
            filters: &op.filters,
            include_ignored: false,
            order: Order::Newest,
            limit: Some(1),
        },
    )?;
    match &outcome {
        Outcome::Entries { entries, .. } if entries.is_empty() => Err(Error::Fail(Fail::NotFound)),
        _ => Ok(outcome),
    }
}

fn today(store: &Store, op: Today) -> Result<Outcome, Error> {
    find_entries(
        store,
        Query {
            schema: &op.schema,
            range: Range::today()?,
            agent: op.agent.as_deref(),
            filters: &op.filters,
            include_ignored: false,
            order: Order::Oldest,
            limit: None,
        },
    )
}

fn list(store: &Store, op: List) -> Result<Outcome, Error> {
    find_entries(
        store,
        Query {
            schema: &op.schema,
            range: op.range,
            agent: op.agent.as_deref(),
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
    filters: &'a [(String, String)],
    include_ignored: bool,
    order: Order,
    limit: Option<usize>,
}

fn find_entries(store: &Store, q: Query<'_>) -> Result<Outcome, Error> {
    let spec = store.load_schema(q.schema)?.spec;
    let resolved = resolve_filters(&spec, q.filters)?;
    let entries = store.find(Find {
        schema: q.schema,
        spec: &spec,
        range: q.range,
        agent: q.agent,
        include_ignored: q.include_ignored,
        filters: &resolved,
        order: q.order,
        limit: q.limit,
    })?;
    Ok(Outcome::Entries { spec, entries })
}

fn sum(store: &Store, op: Sum) -> Result<Outcome, Error> {
    let spec = store.load_schema(&op.schema)?.spec;
    let Some(f) = spec.field(op.field.as_str()) else {
        return Err(Error::Fail(Fail::UnknownField(op.field.clone())));
    };
    if f.type_ != FieldType::Number {
        return Err(Error::Fail(Fail::FieldNotNumber(op.field.clone())));
    }
    let resolved = resolve_filters(&spec, &op.filters)?;
    let entries = store.find(Find {
        schema: &op.schema,
        spec: &spec,
        range: op.range,
        agent: op.agent.as_deref(),
        include_ignored: false,
        filters: &resolved,
        order: Order::Oldest,
        limit: None,
    })?;
    match op.group {
        None => {
            let total: Decimal = entries.iter().filter_map(|e| e.number(&op.field)).sum();
            Ok(Outcome::Total {
                field: op.field,
                value: total,
            })
        }
        Some(Group::Day) => grouped_time(&entries, &op.field, "day"),
        Some(Group::Week) => grouped_time(&entries, &op.field, "week"),
        Some(Group::Month) => grouped_time(&entries, &op.field, "month"),
        Some(Group::Year) => grouped_time(&entries, &op.field, "year"),
        Some(Group::Link(name)) => {
            if spec.field(name.as_str()).is_some() {
                return Err(Error::Fail(Fail::InvalidGroup(name.to_string())));
            }
            grouped_link(&entries, &op.field, name)
        }
    }
}

fn grouped_time(entries: &[Entry], field: &FieldName, unit: &str) -> Result<Outcome, Error> {
    let mut buckets: BTreeMap<String, Decimal> = BTreeMap::new();
    for entry in entries {
        let k = time_group_key(unit, entry.at)?;
        *buckets.entry(k).or_insert(Decimal::ZERO) += entry.number(field).unwrap_or(Decimal::ZERO);
    }
    Ok(Outcome::GroupedTime {
        unit: unit.to_string(),
        buckets: buckets.into_iter().collect(),
    })
}

fn grouped_link(entries: &[Entry], field: &FieldName, name: LinkName) -> Result<Outcome, Error> {
    let mut buckets: BTreeMap<Option<EntryRef>, Decimal> = BTreeMap::new();
    for entry in entries {
        let key = entry
            .links
            .iter()
            .find(|l| l.name.as_str() == name.as_str())
            .map(|l| l.to.clone());
        *buckets.entry(key).or_insert(Decimal::ZERO) +=
            entry.number(field).unwrap_or(Decimal::ZERO);
    }
    Ok(Outcome::GroupedLink {
        name,
        buckets: buckets.into_iter().collect(),
    })
}

fn time_group_key(group: &str, at: Instant) -> Result<String, Error> {
    let date = time::local_civil(at);
    match group {
        "day" => Ok(date.to_string()),
        "month" => Ok(format!("{:04}-{:02}", date.year(), date.month())),
        "year" => Ok(format!("{:04}", date.year())),
        "week" => {
            let iso = date.iso_week_date();
            Ok(format!("{}-W{:02}", iso.year(), iso.week()))
        }
        _ => Err(Error::Fail(Fail::InvalidGroup(group.to_string()))),
    }
}

fn resolve_filters(spec: &Spec, filters: &[(String, String)]) -> Result<Vec<Filter>, Error> {
    let mut out = Vec::new();
    for (name, value) in filters {
        if is_reserved(name) {
            return Err(Error::Usage(Usage::ReservedWhere(name.clone())));
        }
        if let Some(field) = spec.field(name) {
            let value = match field.type_ {
                FieldType::Number => FieldValue::Number(parse_number(value)?),
                FieldType::Enum => FieldValue::Text(fold_enum(value)),
                FieldType::Text => FieldValue::Text(value.clone()),
            };
            out.push(Filter::Field {
                name: FieldName::parse(name)?,
                value,
            });
        } else {
            let link_name = LinkName::parse(name)?;
            let to = EntryRef::parse(value)?;
            out.push(Filter::Link {
                name: link_name,
                to,
            });
        }
    }
    Ok(out)
}

fn ensure_links(store: &Store, spec: &Spec, links: &[Link]) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for link in links {
        if !seen.insert(link.name.as_str()) {
            return Err(Error::Usage(Usage::DuplicateLinkName(link.name.clone())));
        }
        if spec.field(link.name.as_str()).is_some() {
            return Err(Error::Fail(Fail::LinkNameCollidesWithField(
                link.name.clone(),
            )));
        }
        let target = store.load_schema(&link.to.schema)?;
        if store
            .get_entry(&link.to.schema, &target.spec, link.to.id)?
            .is_none()
        {
            return Err(Error::Fail(Fail::LinkTargetMissing(link.to.clone())));
        }
    }
    Ok(())
}

fn prepare_fields(
    spec: &Spec,
    fields: &[(FieldName, String)],
    partial: bool,
) -> Result<HashMap<FieldName, FieldValue>, Error> {
    let mut seen = HashSet::new();
    let mut out = HashMap::new();
    for (name, value) in fields {
        if !seen.insert(name.as_str()) {
            return Err(Error::Usage(Usage::DuplicateField(name.clone())));
        }
        let Some(field) = spec.field(name.as_str()) else {
            return Err(Error::Fail(Fail::UnknownField(name.clone())));
        };
        if value.is_empty() {
            if field.required {
                return Err(Error::Fail(Fail::MissingRequiredField(
                    name.as_str().to_string(),
                )));
            }
            out.insert(name.clone(), FieldValue::Empty);
            continue;
        }
        out.insert(name.clone(), parse_field_value(field, value)?);
    }
    if !partial {
        for field in &spec.fields {
            if field.required && !out.contains_key(field.name.as_str()) {
                return Err(Error::Fail(Fail::MissingRequiredField(field.name.clone())));
            }
        }
    }
    Ok(out)
}

fn parse_field_value(field: &Field, value: &str) -> Result<FieldValue, Error> {
    parse_field_value_parts(&field.name, field.type_, field.values.as_deref(), value)
}

fn parse_field_value_parts(
    name: &str,
    type_: FieldType,
    values: Option<&[String]>,
    value: &str,
) -> Result<FieldValue, Error> {
    match type_ {
        FieldType::Text => {
            if value.contains('\t') || value.contains('\n') {
                return Err(Error::Fail(Fail::TextHasTabOrNewline(name.to_string())));
            }
            Ok(FieldValue::Text(value.to_string()))
        }
        FieldType::Number => Ok(FieldValue::Number(parse_number(value)?)),
        FieldType::Enum => {
            let folded = fold_enum(value);
            let Some(values) = values else {
                return Err(Error::Fail(Fail::EnumHasNoValues(name.to_string())));
            };
            if !values.iter().any(|v| v == &folded) {
                return Err(Error::Fail(Fail::InvalidEnumValue {
                    field: name.to_string(),
                    value: value.to_string(),
                }));
            }
            Ok(FieldValue::Text(folded))
        }
    }
}
