use std::collections::HashMap;
use std::path::PathBuf;

use rust_decimal::Decimal;

use crate::error::{Error, Fail, Usage};
use crate::spec::{
    self, EntryId, EntryRef, EnumValue, Field, FieldKind, FieldName, Group, Link, LinkName,
    SchemaName, Spec, TimePeriod, parse_number,
};
use crate::time::{At, Grain, Period, Range};

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Empty,
    Text(String),
    Number(Decimal),
    Enum(EnumValue),
}

fn parse_text(field: &Field, raw: &str) -> Result<String, Error> {
    if raw.contains('\t') || raw.contains('\n') {
        return Err(Error::Fail(Fail::TextHasTabOrNewline(field.name.clone())));
    }
    Ok(raw.to_string())
}

fn parse_enum(field: &Field, values: &[EnumValue], raw: &str) -> Result<EnumValue, Error> {
    let folded = EnumValue::parse(raw)?;
    if !values.iter().any(|v| v == &folded) {
        return Err(Error::Fail(Fail::InvalidEnumValue {
            field: field.name.clone(),
            value: raw.to_string(),
        }));
    }
    Ok(folded)
}

impl FieldValue {
    pub fn parse(field: &Field, raw: &str) -> Result<Self, Error> {
        if raw.is_empty() {
            return Ok(Self::Empty);
        }
        match &field.kind {
            FieldKind::Text => Ok(Self::Text(parse_text(field, raw)?)),
            FieldKind::Number => Ok(Self::Number(parse_number(raw)?)),
            FieldKind::Enum(values) => Ok(Self::Enum(parse_enum(field, values, raw)?)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent(String);

impl Agent {
    pub fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim_matches(' ');
        if s.is_empty() {
            return Err(Error::Fail(Fail::EmptyAgent));
        }
        if s.contains('\t') || s.contains('\n') {
            return Err(Error::Fail(Fail::AgentHasTabOrNewline));
        }
        Ok(Self(s.to_string()))
    }

    pub fn bottle() -> Self {
        Self("bottle".to_string())
    }
}

spec::string_newtype!(Agent);

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: EntryId,
    pub at: At,
    pub agent: Option<Agent>,
    pub ignored: bool,
    pub values: HashMap<FieldName, FieldValue>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub spec: Spec,
    pub retired: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaInfo {
    pub name: SchemaName,
    pub retired: bool,
}

#[derive(Debug, Clone)]
pub enum NonEmptyFieldValue {
    Text(String),
    Number(Decimal),
    Enum(EnumValue),
}

impl NonEmptyFieldValue {
    pub fn parse(field: &Field, raw: &str) -> Result<Self, Error> {
        match FieldValue::parse(field, raw)? {
            FieldValue::Empty => Err(Error::Usage(Usage::EmptyValue(field.name.clone()))),
            FieldValue::Text(s) => Ok(Self::Text(s)),
            FieldValue::Number(n) => Ok(Self::Number(n)),
            FieldValue::Enum(v) => Ok(Self::Enum(v)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Filter {
    Field {
        name: FieldName,
        value: NonEmptyFieldValue,
    },
    Link {
        name: LinkName,
        to: EntryRef,
    },
}

#[derive(Debug, Clone)]
pub struct FieldInput {
    pub name: FieldName,
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Order {
    Oldest,
    Newest,
}

pub struct Find<'a> {
    pub schema: &'a SchemaName,
    pub spec: &'a Spec,
    pub range: Range,
    pub agent: Option<&'a Agent>,
    pub include_ignored: bool,
    pub filters: &'a [Filter],
    pub order: Order,
    pub limit: Option<usize>,
    /// None means every grain. `Some(g)` keeps instants through `g`.
    pub max_grain: Option<Grain>,
}

pub enum Summed {
    Total(Decimal),
    Time {
        unit: TimePeriod,
        buckets: Vec<(Period, Decimal)>,
    },
    Link {
        name: LinkName,
        buckets: Vec<(Option<EntryRef>, Decimal)>,
    },
}

#[derive(Debug, Clone)]
pub struct SchemaShow {
    pub name: SchemaName,
}

#[derive(Debug, Clone)]
pub struct SchemaAdd {
    pub name: SchemaName,
    pub spec: Spec,
}

#[derive(Debug, Clone)]
pub struct SchemaAddField {
    pub schema: SchemaName,
    pub name: FieldName,
    pub kind: FieldKind,
    pub default: Option<NonEmptyFieldValue>,
}

#[derive(Debug, Clone)]
pub struct SchemaAddValue {
    pub schema: SchemaName,
    pub field: FieldName,
    pub value: EnumValue,
}

#[derive(Debug, Clone)]
pub struct SchemaRenameField {
    pub schema: SchemaName,
    pub from: FieldName,
    pub to: FieldName,
}

#[derive(Debug, Clone)]
pub struct SchemaRetire {
    pub name: SchemaName,
}

#[derive(Debug, Clone)]
pub struct SchemaDrop {
    pub name: SchemaName,
}

#[derive(Debug, Clone)]
pub struct Log {
    pub schema: SchemaName,
    pub at: Option<At>,
    pub agent: Option<Agent>,
    pub links: Vec<Link>,
    pub fields: Vec<FieldInput>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub schema: SchemaName,
    pub agent: Option<Agent>,
    pub fields: Vec<FieldInput>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub struct List {
    pub scope: Scope,
    pub range: Range,
    pub include_ignored: bool,
}

#[derive(Debug, Clone)]
pub struct Get {
    pub schema: SchemaName,
    pub id: EntryId,
}

#[derive(Debug, Clone)]
pub struct Sum {
    pub scope: Scope,
    pub field: FieldName,
    pub range: Range,
    pub group: Option<Group>,
}

#[derive(Debug, Clone)]
pub struct Last {
    pub scope: Scope,
}

#[derive(Debug, Clone)]
pub struct Today {
    pub scope: Scope,
}

#[derive(Debug, Clone)]
pub struct Amend {
    pub schema: SchemaName,
    pub id: EntryId,
    pub at: Option<At>,
    pub agent: Option<Agent>,
    pub links: Vec<Link>,
    pub unlinks: Vec<LinkName>,
    pub fields: Vec<FieldInput>,
}

#[derive(Debug, Clone)]
pub struct Ignore {
    pub schema: SchemaName,
    pub id: EntryId,
}

#[derive(Debug, Clone)]
pub struct Unignore {
    pub schema: SchemaName,
    pub id: EntryId,
}

#[derive(Debug, Clone)]
pub struct Backup {
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum Op {
    SchemaList,
    SchemaShow(SchemaShow),
    SchemaAdd(SchemaAdd),
    SchemaAddField(SchemaAddField),
    SchemaAddValue(SchemaAddValue),
    SchemaRenameField(SchemaRenameField),
    SchemaRetire(SchemaRetire),
    SchemaDrop(SchemaDrop),
    Log(Vec<Log>),
    List(List),
    Get(Get),
    Sum(Sum),
    Last(Last),
    Today(Today),
    Amend(Amend),
    Ignore(Ignore),
    Unignore(Unignore),
    Backup(Backup),
}

#[derive(Debug, Clone)]
pub struct Schemas {
    pub schemas: Vec<SchemaInfo>,
}

#[derive(Debug, Clone)]
pub struct Entries {
    pub spec: Spec,
    pub entries: Vec<Entry>,
    pub include_ignored: bool,
}

#[derive(Debug, Clone)]
pub struct Posted {
    pub id: EntryId,
    pub at: At,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub struct Stamp {
    pub id: EntryId,
    pub at: At,
}

#[derive(Debug, Clone)]
pub struct Total {
    pub field: FieldName,
    pub value: Decimal,
}

#[derive(Debug, Clone)]
pub struct GroupedTime {
    pub unit: TimePeriod,
    pub buckets: Vec<(Period, Decimal)>,
}

#[derive(Debug, Clone)]
pub struct GroupedLink {
    pub name: LinkName,
    pub buckets: Vec<(Option<EntryRef>, Decimal)>,
}

#[derive(Debug, Clone)]
pub enum Outcome {
    Empty,
    Schemas(Schemas),
    Spec(Spec),
    Entries(Entries),
    Posted(Vec<Posted>),
    Stamp(Stamp),
    Total(Total),
    GroupedTime(GroupedTime),
    GroupedLink(GroupedLink),
}
