use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::spec::{EntryRef, FieldName, FieldType, Group, Link, LinkName, SchemaName, Spec};
use crate::time::{Instant, Range};

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Empty,
    Text(String),
    Number(Decimal),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i64,
    pub at: Instant,
    pub agent: Option<String>,
    pub ignored: bool,
    pub values: HashMap<String, FieldValue>,
    pub links: Vec<Link>,
}

impl Entry {
    pub fn number(&self, field: &str) -> Option<Decimal> {
        match self.values.get(field) {
            Some(FieldValue::Number(n)) => Some(*n),
            _ => None,
        }
    }
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

#[derive(Debug, Clone, Copy)]
pub enum Order {
    Oldest,
    Newest,
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
    pub type_: FieldType,
    pub values: Option<Vec<String>>,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchemaAddValue {
    pub schema: SchemaName,
    pub field: FieldName,
    pub value: String,
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
    pub agent: Option<String>,
    pub links: Vec<Link>,
    pub fields: Vec<(FieldName, String)>,
}

#[derive(Debug, Clone)]
pub struct List {
    pub schema: SchemaName,
    pub range: Range,
    pub agent: Option<String>,
    pub filters: Vec<(String, String)>,
    pub include_ignored: bool,
}

#[derive(Debug, Clone)]
pub struct Get {
    pub schema: SchemaName,
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct Sum {
    pub schema: SchemaName,
    pub field: FieldName,
    pub range: Range,
    pub agent: Option<String>,
    pub filters: Vec<(String, String)>,
    pub group: Option<Group>,
}

#[derive(Debug, Clone)]
pub struct Last {
    pub schema: SchemaName,
    pub agent: Option<String>,
    pub filters: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Today {
    pub schema: SchemaName,
    pub agent: Option<String>,
    pub filters: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Amend {
    pub schema: SchemaName,
    pub id: i64,
    pub at: Option<Instant>,
    pub agent: Option<String>,
    pub links: Vec<Link>,
    pub unlinks: Vec<LinkName>,
    pub fields: Vec<(FieldName, String)>,
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
pub enum Outcome {
    Empty,
    Schemas(Vec<SchemaInfo>),
    Spec(Spec),
    Entries {
        spec: Spec,
        entries: Vec<Entry>,
    },
    Posted {
        id: i64,
        at: Instant,
        links: Vec<Link>,
    },
    Stamp {
        id: i64,
        at: Instant,
    },
    Total {
        field: FieldName,
        value: Decimal,
    },
    GroupedTime {
        unit: String,
        buckets: Vec<(String, Decimal)>,
    },
    GroupedLink {
        name: LinkName,
        buckets: Vec<(Option<EntryRef>, Decimal)>,
    },
}
