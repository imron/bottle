use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::error::{Error, Fail};
use crate::spec::{
    self, EntryRef, EnumValue, Field, FieldKind, FieldName, Group, Link, LinkName, SchemaName,
    Spec, TimePeriod, parse_number,
};
use crate::time::{Instant, Period, Range};

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Empty,
    Text(String),
    Number(Decimal),
    Enum(EnumValue),
}

impl FieldValue {
    pub fn parse(field: &Field, raw: &str) -> Result<Self, Error> {
        match &field.kind {
            FieldKind::Text => {
                if raw.contains('\t') || raw.contains('\n') {
                    return Err(Error::Fail(Fail::TextHasTabOrNewline(field.name.clone())));
                }
                Ok(Self::Text(raw.to_string()))
            }
            FieldKind::Number => Ok(Self::Number(parse_number(raw)?)),
            FieldKind::Enum(values) => {
                let folded = EnumValue::parse(raw)?;
                if !values.iter().any(|v| v == &folded) {
                    return Err(Error::Fail(Fail::InvalidEnumValue {
                        field: field.name.clone(),
                        value: raw.to_string(),
                    }));
                }
                Ok(Self::Enum(folded))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent(String);

impl Agent {
    pub fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim_matches(' ');
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
    pub id: i64,
    pub at: Instant,
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
pub enum Filter {
    Field { name: FieldName, value: FieldValue },
    Link { name: LinkName, to: EntryRef },
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
    pub field: Field,
    pub default: Option<FieldValue>,
}

#[derive(Debug, Clone)]
pub struct SchemaAddValue {
    pub schema: SchemaName,
    pub field: FieldName,
    pub value: EnumValue,
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
    pub at: Option<Instant>,
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
    pub id: i64,
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
    pub id: i64,
    pub at: Option<Instant>,
    pub agent: Option<Agent>,
    pub links: Vec<Link>,
    pub unlinks: Vec<LinkName>,
    pub fields: Vec<FieldInput>,
}

#[derive(Debug, Clone)]
pub struct Ignore {
    pub schema: SchemaName,
    pub id: i64,
}

#[derive(Debug, Clone)]
pub enum Op {
    SchemaList,
    SchemaShow(SchemaShow),
    SchemaAdd(SchemaAdd),
    SchemaAddField(SchemaAddField),
    SchemaAddValue(SchemaAddValue),
    SchemaRetire(SchemaRetire),
    SchemaDrop(SchemaDrop),
    Log(Log),
    List(List),
    Get(Get),
    Sum(Sum),
    Last(Last),
    Today(Today),
    Amend(Amend),
    Ignore(Ignore),
}

#[derive(Debug, Clone)]
pub struct Schemas {
    pub schemas: Vec<SchemaInfo>,
}

#[derive(Debug, Clone)]
pub struct Entries {
    pub spec: Spec,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub struct Posted {
    pub id: i64,
    pub at: Instant,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub struct Stamp {
    pub id: i64,
    pub at: Instant,
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
