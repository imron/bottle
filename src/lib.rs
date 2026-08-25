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

use std::path::Path;

pub use db::default_db_path;
pub use error::{Error, Fail, Usage};
pub use input::Cmd;
pub use input::cmd;
pub use ledger::Agent;
pub use spec::FieldType;

pub struct Bottle {
    store: store::Store,
    agent: Agent,
}

impl Bottle {
    pub fn open(path: &Path, agent: Option<String>) -> Result<Self, Error> {
        Ok(Self {
            store: store::Store::open(path)?,
            agent: agent.map(Agent::new).unwrap_or_else(Agent::bottle),
        })
    }
}

pub fn run(db: Option<&Path>, agent: Option<String>, cmd: Cmd) -> Result<String, Error> {
    if let Cmd::Help(help) = &cmd {
        return help::page(help.topic.as_deref());
    }
    let path = db.ok_or(Error::Fail(Fail::DbPathRequired))?;
    let mut bottle = Bottle::open(path, agent)?;
    execute(&mut bottle, cmd)
}

pub fn execute(bottle: &mut Bottle, cmd: Cmd) -> Result<String, Error> {
    if let Cmd::Help(help) = cmd {
        return help::page(help.topic.as_deref());
    }
    let request = input::parse(cmd)?;
    let outcome = domain::execute(&mut bottle.store, &bottle.agent, request.op)?;
    input::render(request.style, request.show_ignored, &outcome)
}
