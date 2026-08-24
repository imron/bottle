use std::collections::{BTreeMap, HashMap, HashSet};

use rust_decimal::Decimal;

use crate::error::{Error, Fail, Usage};
use crate::ledger::{Amend, Entry, FieldValue, Filter, Op, Order, Outcome};
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
        Op::SchemaShow { name } => Ok(Outcome::Spec(store.load_schema(&name)?.spec)),
        Op::SchemaAdd { name, spec } => {
            add_schema(store, name, spec)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaAddField {
            schema,
            name,
            type_,
            values,
            default,
        } => {
            add_field(store, schema, name, type_, values, default)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaAddValue {
            schema,
            field,
            value,
        } => {
            add_value(store, schema, field, value)?;
            Ok(Outcome::Empty)
        }
        Op::SchemaRetire { name } => store.transaction(|tx| {
            retire(tx, &name)?;
            Ok(Outcome::Empty)
        }),
        Op::SchemaDrop { name } => store.transaction(|tx| {
            drop_schema(tx, &name)?;
            Ok(Outcome::Empty)
        }),
        Op::Log {
            schema,
            at,
            agent,
            links,
            fields,
        } => log(store, default_agent, schema, at, agent, links, fields),
        Op::List {
            schema,
            range,
            agent,
            filters,
            include_ignored,
        } => list(
            store,
            Query {
                schema: &schema,
                range,
                agent: agent.as_deref(),
                filters: &filters,
                include_ignored,
                order: Order::Oldest,
                limit: None,
            },
        ),
        Op::Get { schema, id } => get(store, &schema, id),
        Op::Sum {
            schema,
            field,
            range,
            agent,
            filters,
            group,
        } => sum(
            store,
            &schema,
            &field,
            range,
            agent.as_deref(),
            &filters,
            group,
        ),
        Op::Last {
            schema,
            agent,
            filters,
        } => last(store, &schema, agent.as_deref(), &filters),
        Op::Today {
            schema,
            agent,
            filters,
        } => list(
            store,
            Query {
                schema: &schema,
                range: Range::today()?,
                agent: agent.as_deref(),
                filters: &filters,
                include_ignored: false,
                order: Order::Oldest,
                limit: None,
            },
        ),
        Op::Amend { schema, id, change } => amend(store, &schema, id, change),
        Op::Ignore { schema, id } => ignore(store, &schema, id),
    }
}

fn add_schema(store: &mut Store, name: SchemaName, spec: Spec) -> Result<(), Error> {
    store.transaction(|tx| tx.insert_schema(&name, &spec))
}

fn add_field(
    store: &mut Store,
    schema: SchemaName,
    name: FieldName,
    type_: FieldType,
    values: Option<Vec<String>>,
    default: Option<String>,
) -> Result<(), Error> {
    let mut kind = store.load_schema(&schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(schema.clone())));
    }
    if kind.spec.field(name.as_str()).is_some() {
        return Err(Error::Fail(Fail::FieldExists(name.clone())));
    }
    let mut values = values;
    if type_ == FieldType::Enum {
        let Some(vals) = values.as_mut() else {
            return Err(Error::Usage(Usage::EnumValuesRequired));
        };
        fold_enum_values(vals)?;
    } else if values.is_some() {
        return Err(Error::Usage(Usage::EnumValuesNotAllowed));
    }
    let required = default.is_some();
    if let Some(ref def) = default {
        parse_field_value_parts(name.as_str(), type_, values.as_deref(), def)?;
    }
    let field = Field {
        name: name.as_str().to_string(),
        type_,
        required,
        values,
    };
    store.transaction(|tx| {
        tx.add_column(&schema, &field, default.as_deref())?;
        kind.spec.fields.push(field);
        tx.save_spec(&schema, &kind.spec)
    })
}

fn add_value(
    store: &mut Store,
    schema: SchemaName,
    field: FieldName,
    value: String,
) -> Result<(), Error> {
    let mut kind = store.load_schema(&schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(schema.clone())));
    }
    let Some(f) = kind
        .spec
        .fields
        .iter_mut()
        .find(|f| f.name == field.as_str())
    else {
        return Err(Error::Fail(Fail::UnknownField(field.clone())));
    };
    if f.type_ != FieldType::Enum {
        return Err(Error::Fail(Fail::FieldNotEnum(field.clone())));
    }
    let folded = fold_enum(&value);
    let values = f.values.get_or_insert_with(Vec::new);
    if values.iter().any(|v| v == &folded) {
        return Err(Error::Fail(Fail::EnumValueExists(folded)));
    }
    values.push(folded);
    store.transaction(|tx| tx.save_spec(&schema, &kind.spec))
}

fn retire(tx: &mut Tx<'_>, name: &SchemaName) -> Result<(), Error> {
    if tx.retire(name)? == 0 {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    }
    Ok(())
}

fn drop_schema(tx: &mut Tx<'_>, name: &SchemaName) -> Result<(), Error> {
    if !tx.schema_exists(name)? {
        return Err(Error::Fail(Fail::UnknownSchema(name.clone())));
    }
    if tx.inbound_link_count(name)? > 0 {
        return Err(Error::Fail(Fail::SchemaHasInboundLinks(name.clone())));
    }
    tx.drop_schema(name)
}

fn log(
    store: &mut Store,
    default_agent: Option<&str>,
    schema: SchemaName,
    at: Option<Instant>,
    agent: Option<String>,
    mut links: Vec<Link>,
    fields: Vec<(FieldName, String)>,
) -> Result<Outcome, Error> {
    let kind = store.load_schema(&schema)?;
    if kind.retired {
        return Err(Error::Fail(Fail::SchemaRetired(schema.clone())));
    }
    let agent = agent
        .or_else(|| default_agent.map(str::to_string))
        .or_else(|| Some("bottle".to_string()));
    let at = at.unwrap_or_else(Instant::now);
    let values = prepare_fields(&kind.spec, &fields, false)?;
    ensure_links(store, &kind.spec, &links)?;
    links.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    let id = store.transaction(|tx| {
        tx.insert_entry(&schema, &kind.spec, at, agent.as_deref(), &values, &links)
    })?;
    Ok(Outcome::Posted { id, at, links })
}

fn amend(store: &mut Store, schema: &SchemaName, id: i64, change: Amend) -> Result<Outcome, Error> {
    if change.at.is_none()
        && change.agent.is_none()
        && change.links.is_empty()
        && change.unlinks.is_empty()
        && change.fields.is_empty()
    {
        return Err(Error::Usage(Usage::AmendEmpty));
    }
    let kind = store.load_schema(schema)?;
    if store.get_entry(schema, &kind.spec, id)?.is_none() {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: schema.clone(),
            id,
        }));
    }
    let mut unlink_set = HashSet::new();
    for name in &change.unlinks {
        if !unlink_set.insert(name.as_str()) {
            return Err(Error::Usage(Usage::DuplicateUnlink(name.clone())));
        }
        if change
            .links
            .iter()
            .any(|l| l.name.as_str() == name.as_str())
        {
            return Err(Error::Usage(Usage::LinkAndUnlink(name.clone())));
        }
    }
    let values = prepare_fields(&kind.spec, &change.fields, true)?;
    ensure_links(store, &kind.spec, &change.links)?;
    store.transaction(|tx| {
        tx.update_entry(schema, id, change.at, change.agent.as_deref(), &values)?;
        for name in &change.unlinks {
            tx.delete_link(schema, id, name)?;
        }
        let mut links = change.links;
        links.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        for link in &links {
            tx.upsert_link(schema, id, link)?;
        }
        let entry = tx.get_entry(schema, &kind.spec, id)?.ok_or_else(|| {
            Error::Fail(Fail::EntryNotFound {
                schema: schema.clone(),
                id,
            })
        })?;
        Ok(Outcome::Posted {
            id,
            at: entry.at,
            links: entry.links,
        })
    })
}

fn ignore(store: &mut Store, schema: &SchemaName, id: i64) -> Result<Outcome, Error> {
    let spec = store.load_schema(schema)?.spec;
    let Some(entry) = store.get_entry(schema, &spec, id)? else {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: schema.clone(),
            id,
        }));
    };
    store.transaction(|tx| tx.set_ignored(schema, id))?;
    Ok(Outcome::Stamp { id, at: entry.at })
}

fn get(store: &Store, schema: &SchemaName, id: i64) -> Result<Outcome, Error> {
    let spec = store.load_schema(schema)?.spec;
    let Some(entry) = store.get_entry(schema, &spec, id)? else {
        return Err(Error::Fail(Fail::EntryNotFound {
            schema: schema.clone(),
            id,
        }));
    };
    Ok(Outcome::Entries {
        spec,
        entries: vec![entry],
    })
}

fn last(
    store: &Store,
    schema: &SchemaName,
    agent: Option<&str>,
    filters: &[(String, String)],
) -> Result<Outcome, Error> {
    let outcome = list(
        store,
        Query {
            schema,
            range: Range::default(),
            agent,
            filters,
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

struct Query<'a> {
    schema: &'a SchemaName,
    range: Range,
    agent: Option<&'a str>,
    filters: &'a [(String, String)],
    include_ignored: bool,
    order: Order,
    limit: Option<usize>,
}

fn list(store: &Store, q: Query<'_>) -> Result<Outcome, Error> {
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

fn sum(
    store: &Store,
    schema: &SchemaName,
    field: &FieldName,
    range: Range,
    agent: Option<&str>,
    filters: &[(String, String)],
    group: Option<Group>,
) -> Result<Outcome, Error> {
    let spec = store.load_schema(schema)?.spec;
    let Some(f) = spec.field(field.as_str()) else {
        return Err(Error::Fail(Fail::UnknownField(field.clone())));
    };
    if f.type_ != FieldType::Number {
        return Err(Error::Fail(Fail::FieldNotNumber(field.clone())));
    }
    let resolved = resolve_filters(&spec, filters)?;
    let entries = store.find(Find {
        schema,
        spec: &spec,
        range,
        agent,
        include_ignored: false,
        filters: &resolved,
        order: Order::Oldest,
        limit: None,
    })?;
    let key = field.as_str();
    match group {
        None => {
            let total: Decimal = entries.iter().filter_map(|e| e.number(key)).sum();
            Ok(Outcome::Total {
                field: field.clone(),
                value: total,
            })
        }
        Some(Group::Day) => grouped_time(&entries, key, "day"),
        Some(Group::Week) => grouped_time(&entries, key, "week"),
        Some(Group::Month) => grouped_time(&entries, key, "month"),
        Some(Group::Year) => grouped_time(&entries, key, "year"),
        Some(Group::Link(name)) => {
            if spec.field(name.as_str()).is_some() {
                return Err(Error::Fail(Fail::InvalidGroup(name.to_string())));
            }
            grouped_link(&entries, key, name)
        }
    }
}

fn grouped_time(entries: &[Entry], field: &str, unit: &str) -> Result<Outcome, Error> {
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

fn grouped_link(entries: &[Entry], field: &str, name: LinkName) -> Result<Outcome, Error> {
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
) -> Result<HashMap<String, FieldValue>, Error> {
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
            out.insert(name.as_str().to_string(), FieldValue::Empty);
            continue;
        }
        out.insert(name.as_str().to_string(), parse_field_value(field, value)?);
    }
    if !partial {
        for field in &spec.fields {
            if field.required && !out.contains_key(&field.name) {
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
