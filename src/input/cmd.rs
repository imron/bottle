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
    pub name: String,
    #[arg(long)]
    pub yaml: bool,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaAdd {
    pub name: String,
    #[arg(long, required = true)]
    pub file: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaAddField {
    pub schema: String,
    #[arg(long)]
    pub name: String,
    #[arg(long = "type")]
    pub type_: FieldType,
    #[arg(long, value_delimiter = ',')]
    pub values: Option<Vec<String>>,
    #[arg(long)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaAddValue {
    pub schema: String,
    #[arg(long)]
    pub field: String,
    #[arg(long)]
    pub value: String,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaRenameField {
    pub schema: String,
    #[arg(long = "from")]
    pub from: String,
    #[arg(long = "to")]
    pub to: String,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaRetire {
    pub name: String,
}

#[derive(Debug, Clone, Args)]
pub struct SchemaDrop {
    pub name: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SchemaCmd {
    /// List registered schemas
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
    pub schema: String,
    #[arg(long)]
    pub at: Option<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    pub links: Vec<(String, String)>,
    #[arg(trailing_var_arg = true, value_parser = parse_kv)]
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, Args)]
pub struct Filters {
    pub schema: String,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long = "where", value_name = "field=value", value_parser = parse_kv)]
    pub wheres: Vec<(String, String)>,
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    pub links: Vec<(String, String)>,
}

#[derive(Debug, Clone, Args)]
pub struct Ls {
    #[command(flatten)]
    pub filters: Filters,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub include_ignored: bool,
}

#[derive(Debug, Clone, Args)]
pub struct Get {
    pub schema: String,
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
}

#[derive(Debug, Clone, Args)]
pub struct Sum {
    #[command(flatten)]
    pub filters: Filters,
    pub field: String,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct Amend {
    pub schema: String,
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
    #[arg(long)]
    pub at: Option<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long = "link", value_name = "name=SCHEMA/ID", value_parser = parse_kv)]
    pub links: Vec<(String, String)>,
    #[arg(long = "unlink")]
    pub unlinks: Vec<String>,
    #[arg(trailing_var_arg = true, value_parser = parse_kv)]
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, Args)]
pub struct Ignore {
    pub schema: String,
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
}

#[derive(Debug, Clone, Args)]
pub struct Unignore {
    pub schema: String,
    #[arg(value_parser = parse_entry_id)]
    pub id: EntryId,
}

#[derive(Debug, Clone, Args)]
pub struct Backup {
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
