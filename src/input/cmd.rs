use std::path::PathBuf;

use crate::spec::FieldType;

#[derive(Debug, Clone)]
pub struct SchemaShow {
    pub name: String,
    pub yaml: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaAdd {
    pub name: String,
    pub file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SchemaAddField {
    pub schema: String,
    pub name: String,
    pub type_: FieldType,
    pub values: Option<Vec<String>>,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchemaAddValue {
    pub schema: String,
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct SchemaRetire {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct SchemaDrop {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Log {
    pub schema: String,
    pub at: Option<String>,
    pub agent: Option<String>,
    pub links: Vec<(String, String)>,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Ls {
    pub schema: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub agent: Option<String>,
    pub wheres: Vec<(String, String)>,
    pub links: Vec<(String, String)>,
    pub include_ignored: bool,
}

#[derive(Debug, Clone)]
pub struct Get {
    pub schema: String,
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct Sum {
    pub schema: String,
    pub field: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub agent: Option<String>,
    pub wheres: Vec<(String, String)>,
    pub links: Vec<(String, String)>,
    pub group: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Last {
    pub schema: String,
    pub agent: Option<String>,
    pub wheres: Vec<(String, String)>,
    pub links: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Today {
    pub schema: String,
    pub agent: Option<String>,
    pub wheres: Vec<(String, String)>,
    pub links: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Amend {
    pub schema: String,
    pub id: i64,
    pub at: Option<String>,
    pub agent: Option<String>,
    pub links: Vec<(String, String)>,
    pub unlinks: Vec<String>,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Ignore {
    pub schema: String,
    pub id: i64,
}

#[derive(Debug, Clone)]
pub enum Cmd {
    SchemaList,
    SchemaShow(SchemaShow),
    SchemaAdd(SchemaAdd),
    SchemaAddField(SchemaAddField),
    SchemaAddValue(SchemaAddValue),
    SchemaRetire(SchemaRetire),
    SchemaDrop(SchemaDrop),
    Log(Log),
    Ls(Ls),
    Get(Get),
    Sum(Sum),
    Last(Last),
    Today(Today),
    Amend(Amend),
    Ignore(Ignore),
}
