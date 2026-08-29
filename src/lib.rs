mod db;
mod domain;
mod error;
mod input;
mod ledger;
mod mutable_store;
mod output;
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
pub use input::parse::parse;
pub use ledger::{Agent, Op};
pub use output::Style;
pub use spec::FieldType;

pub struct Bottle {
    db: db::Db,
    agent: Agent,
    tz: TimeZone,
}

impl Bottle {
    pub fn open(path: &Path, agent: Option<String>, tz: Option<&str>) -> Result<Self, Error> {
        let tz = time::zone(tz)?;
        Ok(Self {
            db: db::Db::open(path, tz.clone())?,
            agent: resolve_agent(
                agent.as_deref(),
                std::env::var("BOTTLE_AGENT").ok().as_deref(),
            )?,
            tz,
        })
    }

    pub fn tz(&self) -> &TimeZone {
        &self.tz
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

pub async fn mcp(path: &Path, agent: Option<String>, tz: Option<&str>) -> Result<(), Error> {
    input::mcp::serve(path, agent, tz).await
}

pub fn style(cmd: &Cmd) -> Style {
    match cmd {
        Cmd::Schema(cmd::SchemaCmd::Show(show)) if show.yaml => Style::Yaml,
        _ => Style::Tsv,
    }
}

pub fn execute(bottle: &mut Bottle, op: Op, style: Style) -> Result<String, Error> {
    let outcome = domain::execute(&mut bottle.db, &bottle.agent, &bottle.tz, op)?;
    output::render(&outcome, &bottle.tz, style)
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
        assert!(matches!(
            resolve_agent(None, Some("")).unwrap_err(),
            Error::Fail(Fail::EmptyAgent)
        ));
        assert!(matches!(
            resolve_agent(None, Some("   ")).unwrap_err(),
            Error::Fail(Fail::EmptyAgent)
        ));
        assert!(matches!(
            resolve_agent(Some(""), Some("env")).unwrap_err(),
            Error::Fail(Fail::EmptyAgent)
        ));
    }
}
