mod cmd;
mod db;
mod domain;
mod error;
mod help;
mod input;
mod ledger;
mod mutable_store;
mod spec;
mod sql;
mod store;
mod time;
mod tsv;

use std::path::Path;

pub use cmd::Cmd;
pub use db::default_db_path;
pub use error::Error;
pub use spec::FieldType;

pub struct Bottle {
    store: store::Store,
    default_agent: Option<String>,
}

impl Bottle {
    pub fn open(path: &Path, default_agent: Option<String>) -> Result<Self, Error> {
        Ok(Self {
            store: store::Store::open(path)?,
            default_agent,
        })
    }
}

pub fn run(db: Option<&Path>, default_agent: Option<String>, cmd: Cmd) -> Result<String, Error> {
    if let Cmd::Help { topic } = &cmd {
        return help::page(topic.as_deref());
    }
    let path = db.ok_or_else(|| Error::fail("db path required"))?;
    let mut bottle = Bottle::open(path, default_agent)?;
    execute(&mut bottle, cmd)
}

pub fn execute(bottle: &mut Bottle, cmd: Cmd) -> Result<String, Error> {
    if let Cmd::Help { topic } = cmd {
        return help::page(topic.as_deref());
    }
    let request = input::parse(cmd)?;
    let outcome = domain::execute(
        &mut bottle.store,
        bottle.default_agent.as_deref(),
        request.op,
    )?;
    input::render(request.style, request.show_ignored, &outcome)
}
