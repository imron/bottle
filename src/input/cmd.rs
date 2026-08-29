use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::spec::{EntryId, FieldType};

fn parse_kv(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((name, value)) if !name.is_empty() => Ok((name.to_string(), value.to_string())),
        _ => Err("expected name=value".into()),
    }
}

fn parse_entry_id(s: &str) -> Result<EntryId, String> {
    let n: i64 = s.parse().map_err(|_| format!("invalid id: {s}"))?;
    EntryId::parse(n).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Args)]
pub struct SchemaShow {
    /// Schema name
    pub name: String,
    /// Print the YAML field list
    #[arg(long)]
    pub yaml: bool,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaAdd {
    /// Schema name
    pub name: String,
    /// Path to the YAML spec
    #[arg(long, required = true, value_name = "PATH")]
    pub file: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaAddField {
    /// Schema name
    pub schema: String,
    /// Field name
    #[arg(long)]
    pub name: String,
    /// Field type
    #[arg(long = "type")]
    pub type_: FieldType,
    /// Enum values, comma-separated
    #[arg(long, value_delimiter = ',', value_name = "a,b")]
    pub values: Option<Vec<String>>,
    /// Backfill existing entries and make the field required
    #[arg(long, value_name = "VALUE")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaAddValue {
    /// Schema name
    pub schema: String,
    /// Enum field name
    #[arg(long)]
    pub field: String,
    /// Value to append
    #[arg(long)]
    pub value: String,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaRename {
    /// Current schema name
    pub from: String,
    /// New schema name
    pub to: String,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaRenameField {
    /// Schema name
    pub schema: String,
    /// Current field name
    #[arg(long = "from")]
    pub from: String,
    /// New field name
    #[arg(long = "to")]
    pub to: String,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaRetire {
    /// Schema name
    pub name: String,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaDrop {
    /// Schema name
    pub name: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SchemaCmd {
    /// List registered schemas
    #[command(visible_alias = "ls")]
    List,
    /// Print the field list of a schema
    Show(SchemaShow),
    /// Register a type from a YAML file
    Add(SchemaAdd),
    /// Add one field to an existing schema
    #[command(name = "add-field")]
    AddField(SchemaAddField),
    /// Append one value to an enum field
    #[command(name = "add-value")]
    AddValue(SchemaAddValue),
    /// Rename a schema
    Rename(SchemaRename),
    /// Rename a field on an existing schema
    #[command(name = "rename-field")]
    RenameField(SchemaRenameField),
    /// Block new logs on a schema
    Retire(SchemaRetire),
    /// Delete a schema and its entries
    Drop(SchemaDrop),
}

#[derive(Debug, Clone, Args)]
pub struct Log {
    /// Schema name
    pub schema: String,
    /// Time of the event; omit to use now. A date is a day, YYYY-MM is a month
    #[arg(long, value_name = "DATE|TIME")]
    pub at: Option<String>,
    /// Who wrote the entry
    #[arg(long, value_name = "NAME")]
    pub agent: Option<String>,
    /// Named pointer to another entry
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    pub links: Vec<(String, String)>,
    /// TSV file of entries; - is stdin
    #[arg(long, value_name = "PATH")]
    pub file: Option<PathBuf>,
    /// Declared fields as name=value
    #[arg(trailing_var_arg = true, value_name = "name=value", value_parser = parse_kv)]
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, Args)]
pub struct Filters {
    /// Schema name
    pub schema: String,
    /// Only entries written by that agent
    #[arg(long, value_name = "NAME")]
    pub agent: Option<String>,
    /// Filter a declared field; repeat, all AND
    #[arg(long = "where", value_name = "field=value", value_parser = parse_kv)]
    pub wheres: Vec<(String, String)>,
    /// Drop a row matching this field equality; repeat, any match drops
    #[arg(long = "exclude", value_name = "field=value", value_parser = parse_kv)]
    pub excludes: Vec<(String, String)>,
    /// Filter a named pointer; repeat, all AND
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    pub links: Vec<(String, String)>,
}

#[derive(Debug, Clone, Args)]
pub struct Ls {
    #[command(flatten)]
    pub filters: Filters,
    /// Lower bound on at
    #[arg(long, value_name = "DATE|TIME")]
    pub from: Option<String>,
    /// Upper bound on at
    #[arg(long, value_name = "DATE|TIME")]
    pub to: Option<String>,
    /// Include ignored entries and print the ignored column
    #[arg(long)]
    pub include_ignored: bool,
}

#[derive(Debug, Clone, Args)]
pub struct Get {
    /// Schema name
    pub schema: String,
    /// Entry id
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
}

#[derive(Debug, Clone, Args)]
pub struct Sum {
    #[command(flatten)]
    pub filters: Filters,
    /// Number field to total
    pub field: String,
    /// Lower bound on at
    #[arg(long, value_name = "DATE|TIME")]
    pub from: Option<String>,
    /// Upper bound on at
    #[arg(long, value_name = "DATE|TIME")]
    pub to: Option<String>,
    /// Bucket by day, week, month, year, or a link name
    #[arg(long, value_name = "day|week|month|year|LINK")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct Amend {
    /// Schema name
    pub schema: String,
    /// Entry id
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
    /// Time of the event. A date is a day, YYYY-MM is a month
    #[arg(long, value_name = "DATE|TIME")]
    pub at: Option<String>,
    /// Set who wrote the entry
    #[arg(long, value_name = "NAME")]
    pub agent: Option<String>,
    /// Set or replace a named pointer
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    pub links: Vec<(String, String)>,
    /// Remove a named pointer
    #[arg(long = "unlink", value_name = "NAME")]
    pub unlinks: Vec<String>,
    /// Fields to change as name=value
    #[arg(trailing_var_arg = true, value_name = "name=value", value_parser = parse_kv)]
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, Args)]
pub struct Ignore {
    /// Schema name
    pub schema: String,
    /// Entry id
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
}

#[derive(Debug, Clone, Args)]
pub struct Unignore {
    /// Schema name
    pub schema: String,
    /// Entry id
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
}

#[derive(Debug, Clone, Args)]
pub struct Backup {
    /// Destination sqlite file
    pub path: PathBuf,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Cmd {
    /// Declare and change types of entry
    #[command(subcommand)]
    Schema(SchemaCmd),
    /// Write one entry of a registered schema
    Log(Log),
    /// List entries of a schema
    #[command(visible_alias = "list")]
    Ls(Ls),
    /// Print one entry by schema and id
    Get(Get),
    /// Total a number field
    Sum(Sum),
    /// Print the most recent entry of a schema
    Last(Filters),
    /// List entries for the current civil day
    Today(Filters),
    /// Change an existing entry in place
    Amend(Amend),
    /// Hide an entry from lists and totals
    Ignore(Ignore),
    /// Show an ignored entry in lists and totals again
    Unignore(Unignore),
    /// Copy the ledger to a sqlite file
    Backup(Backup),
}
