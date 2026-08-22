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
pub use store::Bottle;

pub fn run(db: Option<&Path>, default_agent: Option<String>, cmd: Cmd) -> Result<String, Error> {
    if let Cmd::Help { topic } = &cmd {
        return help::page(topic.as_deref());
    }
    let path = db.ok_or_else(|| Error::fail("db path required"))?;
    let mut bottle = Bottle::open(path, default_agent)?;
    bottle.run(cmd)
}
