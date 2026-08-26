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
            agent: resolve_agent(
                agent.as_deref(),
                std::env::var("BOTTLE_AGENT").ok().as_deref(),
            )?,
            tz: time::zone(tz)?,
        })
    }
}

fn resolve_agent(explicit: Option<&str>, env: Option<&str>) -> Result<Agent, Error> {
    match explicit.or(env) {
        Some(s) => Agent::parse(s),
        None => Ok(Agent::bottle()),
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

pub fn log_entries(bottle: &mut Bottle, logs: Vec<cmd::Log>) -> Result<String, Error> {
    let mut ops = Vec::new();
    for log in logs {
        ops.push(input::log(log, &bottle.tz)?);
    }
    let outcome = domain::log_many(&mut bottle.db, &bottle.agent, ops)?;
    input::render(input::Style::Tsv, false, &outcome, &bottle.tz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_prefers_explicit_then_env_then_bottle() {
        assert_eq!(
            resolve_agent(Some("cli"), Some("env")).unwrap().as_str(),
            "cli"
        );
        assert_eq!(resolve_agent(None, Some("env")).unwrap().as_str(), "env");
        assert_eq!(resolve_agent(None, None).unwrap().as_str(), "bottle");
        assert_eq!(
            resolve_agent(None, Some("  bot  ")).unwrap().as_str(),
            "bot"
        );
    }
}
