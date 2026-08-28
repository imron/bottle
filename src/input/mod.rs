pub mod cmd;
mod error;
pub mod help;
pub mod mcp;
mod parse;

use std::path::PathBuf;

pub use cmd::Cmd;
pub use parse::{
    amend, get, ignore, last, log, ls, parse, schema_add, schema_add_field, schema_add_value,
    schema_drop, schema_retire, schema_show, sum, today,
};

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
    pub id: i64,
    pub at: Option<String>,
    pub agent: Option<String>,
    pub links: Vec<(String, String)>,
    pub unlinks: Vec<String>,
    pub fields: Vec<(String, String)>,
}
