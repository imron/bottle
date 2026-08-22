mod cmd;
mod db;
mod error;
mod help;
mod mutable_store;
mod spec;
mod store;
mod time;
mod tsv;
mod value;

use std::path::Path;

pub use cmd::Cmd;
pub use db::default_db_path;
pub use error::Error;
pub use spec::FieldType;
use store::Amend;
pub use store::Bottle;
use time::Range;

pub fn run(db: Option<&Path>, default_agent: Option<String>, cmd: Cmd) -> Result<String, Error> {
    if let Cmd::Help { topic } = &cmd {
        return help::page(topic.as_deref());
    }
    let path = db.ok_or_else(|| Error::fail("db path required"))?;
    let mut bottle = Bottle::open(path, default_agent)?;
    execute(&mut bottle, cmd)
}

pub fn execute(bottle: &mut Bottle, cmd: Cmd) -> Result<String, Error> {
    match cmd {
        Cmd::Help { topic } => help::page(topic.as_deref()),
        Cmd::SchemaList => bottle.schema_list(),
        Cmd::SchemaShow { name, yaml } => bottle.schema_show(&name, yaml),
        Cmd::SchemaAdd { name, file } => bottle.transaction(|tx| tx.schema_add(&name, &file)),
        Cmd::SchemaAddField {
            schema,
            name,
            type_,
            values,
            default,
        } => bottle.transaction(|tx| tx.schema_add_field(&schema, &name, type_, values, default)),
        Cmd::SchemaAddValue {
            schema,
            field,
            value,
        } => bottle.transaction(|tx| tx.schema_add_value(&schema, &field, &value)),
        Cmd::SchemaRetire { name } => bottle.transaction(|tx| tx.schema_retire(&name)),
        Cmd::SchemaDrop { name } => bottle.transaction(|tx| tx.schema_drop(&name)),
        Cmd::Log {
            schema,
            at,
            agent,
            links,
            fields,
        } => bottle
            .transaction(|tx| tx.log(&schema, at.as_deref(), agent.as_deref(), &links, &fields)),
        Cmd::Ls {
            schema,
            from,
            to,
            agent,
            wheres,
            include_ignored,
        } => {
            let range = Range::parse(from.as_deref(), to.as_deref())?;
            bottle.ls(&schema, range, agent.as_deref(), &wheres, include_ignored)
        }
        Cmd::Get { schema, id } => bottle.get(&schema, id),
        Cmd::Sum {
            schema,
            field,
            from,
            to,
            agent,
            wheres,
            group,
        } => {
            let range = Range::parse(from.as_deref(), to.as_deref())?;
            bottle.sum(
                &schema,
                &field,
                range,
                agent.as_deref(),
                &wheres,
                group.as_deref(),
            )
        }
        Cmd::Last {
            schema,
            agent,
            wheres,
        } => bottle.last(&schema, agent.as_deref(), &wheres),
        Cmd::Today {
            schema,
            agent,
            wheres,
        } => bottle.today(&schema, agent.as_deref(), &wheres),
        Cmd::Amend {
            schema,
            id,
            at,
            agent,
            links,
            unlinks,
            fields,
        } => bottle.transaction(|tx| {
            tx.amend(
                &schema,
                id,
                Amend {
                    at: at.as_deref(),
                    agent: agent.as_deref(),
                    links: &links,
                    unlinks: &unlinks,
                    fields: &fields,
                },
            )
        }),
        Cmd::Ignore { schema, id } => bottle.transaction(|tx| tx.ignore(&schema, id)),
    }
}
