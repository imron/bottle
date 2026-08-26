mod db;
mod domain;
mod error;
mod input;
mod ledger;
mod mutable_store;
mod spec;
mod sql;
mod store;
mod time;

use std::path::Path;

use jiff::tz::TimeZone;

pub use db::default_db_path;
pub use error::{Error, Fail, Usage};
pub use input::Cmd;
pub use input::cmd;
pub use ledger::Agent;
pub use spec::FieldType;

pub struct Bottle {
    db: db::Db,
    agent: Agent,
    tz: TimeZone,
}

impl Bottle {
    pub fn open(path: &Path, agent: Option<String>, tz: Option<&str>) -> Result<Self, Error> {
        Ok(Self {
            db: db::Db::open(path)?,
            agent: agent.map(Agent::new).unwrap_or_else(Agent::bottle),
            tz: time::zone(tz)?,
        })
    }
}

pub fn help(topic: Option<&str>) -> Result<String, Error> {
    input::help::page(topic)
}

pub fn run(
    db: Option<&Path>,
    agent: Option<String>,
    tz: Option<&str>,
    cmd: Cmd,
) -> Result<String, Error> {
    let path = db.ok_or(Error::Fail(Fail::DbPathRequired))?;
    let mut bottle = Bottle::open(path, agent, tz)?;
    execute(&mut bottle, cmd)
}

pub fn execute(bottle: &mut Bottle, cmd: Cmd) -> Result<String, Error> {
    let request = input::parse(cmd, &bottle.tz)?;
    let outcome = domain::execute(&mut bottle.db, &bottle.agent, &bottle.tz, request.op)?;
    input::render(request.style, request.show_ignored, &outcome, &bottle.tz)
}
