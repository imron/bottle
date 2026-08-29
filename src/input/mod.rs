pub mod cmd;
pub mod help;
pub mod mcp;
pub mod parse;
pub mod tsv;

use std::path::PathBuf;

use crate::spec::EntryId;

pub use cmd::Cmd;

pub enum SpecSource {
    File(PathBuf),
    Yaml(String),
}

pub struct ScopeInput {
    pub schema: String,
    pub agent: Option<String>,
    pub wheres: Vec<(String, String)>,
    pub links: Vec<(String, String)>,
}

impl From<cmd::Filters> for ScopeInput {
    fn from(filters: cmd::Filters) -> Self {
        Self {
            schema: filters.schema,
            agent: filters.agent,
            wheres: filters.wheres,
            links: filters.links,
        }
    }
}

pub struct AmendInput {
    pub schema: String,
    pub id: EntryId,
    pub at: Option<String>,
    pub agent: Option<String>,
    pub links: Vec<(String, String)>,
    pub unlinks: Vec<String>,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct LogInput {
    pub schema: String,
    pub at: Option<String>,
    pub agent: Option<String>,
    pub links: Vec<(String, String)>,
    pub fields: Vec<(String, String)>,
}

impl From<cmd::Log> for LogInput {
    fn from(cmd: cmd::Log) -> Self {
        Self {
            schema: cmd.schema,
            at: cmd.at,
            agent: cmd.agent,
            links: cmd.links,
            fields: cmd.fields,
        }
    }
}
