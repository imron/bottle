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
pub struct Amend {
    pub at: Option<Instant>,
    pub agent: Option<String>,
    pub links: Vec<Link>,
    pub unlinks: Vec<LinkName>,
    pub fields: Vec<(FieldName, String)>,
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
pub enum Op {
    SchemaList,
    SchemaShow {
        name: SchemaName,
    },
    SchemaAdd {
        name: SchemaName,
        spec: Spec,
    },
    SchemaAddField {
        schema: SchemaName,
        name: FieldName,
        type_: FieldType,
        values: Option<Vec<String>>,
        default: Option<String>,
    },
    SchemaAddValue {
        schema: SchemaName,
        field: FieldName,
        value: String,
    },
    SchemaRetire {
        name: SchemaName,
    },
    SchemaDrop {
        name: SchemaName,
    },
    Log {
        schema: SchemaName,
        at: Option<Instant>,
        agent: Option<String>,
        links: Vec<Link>,
        fields: Vec<(FieldName, String)>,
    },
    List {
        schema: SchemaName,
        range: Range,
        agent: Option<String>,
        filters: Vec<(String, String)>,
        include_ignored: bool,
    },
    Get {
        schema: SchemaName,
        id: i64,
    },
    Sum {
        schema: SchemaName,
        field: FieldName,
        range: Range,
        agent: Option<String>,
        filters: Vec<(String, String)>,
        group: Option<Group>,
    },
    Last {
        schema: SchemaName,
        agent: Option<String>,
        filters: Vec<(String, String)>,
    },
    Today {
        schema: SchemaName,
        agent: Option<String>,
        filters: Vec<(String, String)>,
    },
    Amend {
        schema: SchemaName,
        id: i64,
        change: Amend,
    },
    Ignore {
        schema: SchemaName,
        id: i64,
    },
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
