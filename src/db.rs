use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::error::Error;

pub fn default_db_path() -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os("BOTTLE_DB") {
        return Ok(PathBuf::from(path));
    }
    if cfg!(target_os = "macos") {
        let home = home_dir()?;
        return Ok(home.join(".config/bottle/bottle.db"));
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg).join("bottle/bottle.db"));
    }
    if let Some(dirs) = directories::BaseDirs::new() {
        return Ok(dirs.data_dir().join("bottle/bottle.db"));
    }
    let home = home_dir()?;
    Ok(home.join(".local/share/bottle/bottle.db"))
}

pub fn open(path: &Path) -> Result<Connection, Error> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schemas (
            name    TEXT PRIMARY KEY,
            spec    TEXT NOT NULL,
            retired INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS links (
            from_schema TEXT NOT NULL,
            from_id     INTEGER NOT NULL,
            name        TEXT NOT NULL,
            to_schema   TEXT NOT NULL,
            to_id       INTEGER NOT NULL,
            PRIMARY KEY (from_schema, from_id, name)
         );",
    )?;
    Ok(conn)
}

fn home_dir() -> Result<PathBuf, Error> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::fail("HOME is not set"))
}
