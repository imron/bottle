use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection as Sqlite;
use rusqlite::OptionalExtension;
use rusqlite::TransactionBehavior;
use rusqlite::functions::{Aggregate, FunctionFlags};
use rust_decimal::Decimal;

use crate::error::{Error, Fail};
use crate::spec::{FieldKind, SchemaName, Spec, TimePeriod};
use crate::sql::{StoredNumber, StoredSchemaName, StoredTime};
use crate::time::{self, Instant};
use jiff::tz::TimeZone;

pub(crate) const USER_VERSION: i32 = 2;

const CATALOG_SQL: &str = "
CREATE TABLE IF NOT EXISTS schemas (
    name    TEXT PRIMARY KEY,
    retired INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS schema_fields (
    schema   TEXT NOT NULL,
    position INTEGER NOT NULL,
    name     TEXT NOT NULL,
    kind     TEXT NOT NULL,
    required INTEGER NOT NULL,
    PRIMARY KEY (schema, name),
    UNIQUE (schema, position)
);
CREATE TABLE IF NOT EXISTS schema_enum_values (
    schema   TEXT NOT NULL,
    field    TEXT NOT NULL,
    position INTEGER NOT NULL,
    value    TEXT NOT NULL,
    PRIMARY KEY (schema, field, value),
    UNIQUE (schema, field, position)
);
CREATE TABLE IF NOT EXISTS links (
    from_schema TEXT NOT NULL,
    from_id     INTEGER NOT NULL,
    name        TEXT NOT NULL,
    to_schema   TEXT NOT NULL,
    to_id       INTEGER NOT NULL,
    PRIMARY KEY (from_schema, from_id, name)
);
";

pub trait UniqueConstraint<T> {
    fn unique(self, err: Fail) -> Result<T, Error>;
}

impl<T> UniqueConstraint<T> for Result<T, rusqlite::Error> {
    fn unique(self, err: Fail) -> Result<T, Error> {
        self.map_err(|e| {
            if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                Error::Fail(err)
            } else {
                Error::from(e)
            }
        })
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        // UDF failures arrive as SqliteFailure plus the message.
        // sqlite3_result_error drops the boxed Error, so this cannot
        // downcast UserFunctionError back to Fail::CorruptStored*.
        Self::Fail(Fail::Store(err.to_string()))
    }
}

impl From<Error> for rusqlite::Error {
    fn from(err: Error) -> Self {
        Self::UserFunctionError(Box::new(err))
    }
}

/// Readable sqlite session. Implemented by [`Db`] and [`Tx`].
pub trait Connection: AsRef<Sqlite> {}

pub struct Db {
    conn: Sqlite,
}

pub struct Tx<'a> {
    inner: rusqlite::Transaction<'a>,
}

impl Db {
    pub fn open(path: &Path, tz: TimeZone) -> Result<Self, Error> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Sqlite::open(path)?;
        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let v: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if v > USER_VERSION {
            return Err(Error::Fail(Fail::UnsupportedStoreVersion(v)));
        }
        if v < 2 {
            migrate_yaml_catalog(&conn)?;
        }
        conn.execute_batch(CATALOG_SQL)?;
        if v < USER_VERSION {
            conn.pragma_update(None, "user_version", USER_VERSION)?;
        }
        register_functions(&conn, tz)?;
        Ok(Self { conn })
    }

    pub fn transaction<T>(
        &mut self,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut tx = Tx::begin(&mut self.conn)?;
        match f(&mut tx) {
            Ok(value) => {
                tx.commit()?;
                Ok(value)
            }
            Err(err) => Err(err),
        }
    }

    pub fn read<T>(&mut self, f: impl FnOnce(&Tx<'_>) -> Result<T, Error>) -> Result<T, Error> {
        let tx = Tx::begin_deferred(&mut self.conn)?;
        match f(&tx) {
            Ok(value) => {
                tx.commit()?;
                Ok(value)
            }
            Err(err) => Err(err),
        }
    }
}

impl AsRef<Sqlite> for Db {
    fn as_ref(&self) -> &Sqlite {
        &self.conn
    }
}

impl Connection for Db {}

impl<'a> Tx<'a> {
    fn begin(conn: &'a mut Sqlite) -> Result<Self, Error> {
        Ok(Self {
            inner: conn.transaction_with_behavior(TransactionBehavior::Immediate)?,
        })
    }

    fn begin_deferred(conn: &'a mut Sqlite) -> Result<Self, Error> {
        Ok(Self {
            inner: conn.transaction_with_behavior(TransactionBehavior::Deferred)?,
        })
    }

    fn commit(self) -> Result<(), Error> {
        self.inner.commit()?;
        Ok(())
    }
}

impl AsRef<Sqlite> for Tx<'_> {
    fn as_ref(&self) -> &Sqlite {
        &self.inner
    }
}

impl Connection for Tx<'_> {}

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

fn register_functions(conn: &Sqlite, tz: TimeZone) -> Result<(), Error> {
    conn.create_scalar_function(
        "bottle_dec_eq",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let left: Option<String> = ctx.get(0)?;
            let right: Option<String> = ctx.get(1)?;
            Ok(dec_eq(left.as_deref(), right.as_deref())?)
        },
    )?;
    conn.create_aggregate_function(
        "bottle_dec_sum",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        DecSum,
    )?;
    conn.create_scalar_function(
        "bottle_period",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            let at: String = ctx.get(0)?;
            let unit: String = ctx.get(1)?;
            Ok(period_bucket(&at, &unit, &tz)?)
        },
    )?;
    Ok(())
}

fn period_bucket(at: &str, unit: &str, tz: &TimeZone) -> Result<String, Error> {
    let at = Instant::try_from(StoredTime(at.to_string()))?;
    let Some(unit) = TimePeriod::parse(unit) else {
        return Err(Error::Fail(Fail::Store(format!("unknown period: {unit}"))));
    };
    Ok(time::period(unit, at, tz).to_string())
}

struct DecSum;

impl Aggregate<Decimal, String> for DecSum {
    fn init(&self, _ctx: &mut rusqlite::functions::Context<'_>) -> rusqlite::Result<Decimal> {
        Ok(Decimal::ZERO)
    }

    fn step(
        &self,
        ctx: &mut rusqlite::functions::Context<'_>,
        acc: &mut Decimal,
    ) -> rusqlite::Result<()> {
        let v: Option<String> = ctx.get(0)?;
        *acc = dec_add(*acc, v.as_deref())?;
        Ok(())
    }

    fn finalize(
        &self,
        _ctx: &mut rusqlite::functions::Context<'_>,
        acc: Option<Decimal>,
    ) -> rusqlite::Result<String> {
        Ok(acc.unwrap_or(Decimal::ZERO).to_string())
    }
}

fn dec_add(acc: Decimal, raw: Option<&str>) -> Result<Decimal, Error> {
    let Some(raw) = raw else {
        return Ok(acc);
    };
    if raw.is_empty() {
        return Ok(acc);
    }
    let n = Decimal::try_from(StoredNumber(raw.to_string()))?;
    acc.checked_add(n).ok_or(Error::Fail(Fail::NumberOverflow))
}

fn dec_eq(left: Option<&str>, right: Option<&str>) -> Result<bool, Error> {
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(false);
    };
    if left.is_empty() || right.is_empty() {
        return Ok(false);
    }
    let left = Decimal::try_from(StoredNumber(left.to_string()))?;
    let right = Decimal::try_from(StoredNumber(right.to_string()))?;
    Ok(left == right)
}

fn table_exists(conn: &Sqlite, name: &str) -> Result<bool, Error> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

fn schemas_has_spec(conn: &Sqlite) -> Result<bool, Error> {
    let mut stmt = conn.prepare("PRAGMA table_info(schemas)")?;
    let mut raw = stmt.query([])?;
    while let Some(r) = raw.next()? {
        let name: String = r.get(1)?;
        if name == "spec" {
            return Ok(true);
        }
    }
    Ok(false)
}

fn migrate_yaml_catalog(conn: &Sqlite) -> Result<(), Error> {
    if !table_exists(conn, "schemas")? {
        return Ok(());
    }
    if !schemas_has_spec(conn)? {
        return Ok(());
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_fields (
            schema   TEXT NOT NULL,
            position INTEGER NOT NULL,
            name     TEXT NOT NULL,
            kind     TEXT NOT NULL,
            required INTEGER NOT NULL,
            PRIMARY KEY (schema, name),
            UNIQUE (schema, position)
         );
         CREATE TABLE IF NOT EXISTS schema_enum_values (
            schema   TEXT NOT NULL,
            field    TEXT NOT NULL,
            position INTEGER NOT NULL,
            value    TEXT NOT NULL,
            PRIMARY KEY (schema, field, value),
            UNIQUE (schema, field, position)
         );",
    )?;
    let rows = {
        let mut stmt = conn.prepare("SELECT name, spec FROM schemas")?;
        let mut raw = stmt.query([])?;
        let mut rows = Vec::new();
        while let Some(r) = raw.next()? {
            let name: String = r.get(0)?;
            let spec: String = r.get(1)?;
            rows.push((name, spec));
        }
        rows
    };
    for (name, yaml) in rows {
        let spec = Spec::parse_yaml(&yaml)?;
        let name = SchemaName::try_from(StoredSchemaName(name))?;
        insert_migrated_spec(conn, &name, &spec)?;
    }
    conn.execute_batch(
        "CREATE TABLE schemas_v2 (
            name    TEXT PRIMARY KEY,
            retired INTEGER NOT NULL DEFAULT 0
         );
         INSERT INTO schemas_v2 (name, retired) SELECT name, retired FROM schemas;
         DROP TABLE schemas;
         ALTER TABLE schemas_v2 RENAME TO schemas;",
    )?;
    Ok(())
}

fn insert_migrated_spec(conn: &Sqlite, name: &SchemaName, spec: &Spec) -> Result<(), Error> {
    for (i, field) in spec.fields.iter().enumerate() {
        let kind = match &field.kind {
            FieldKind::Text => "text",
            FieldKind::Number => "number",
            FieldKind::Enum(_) => "enum",
        };
        conn.execute(
            "INSERT INTO schema_fields (schema, position, name, kind, required)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                name.as_str(),
                i as i64,
                field.name.as_str(),
                kind,
                field.required as i64
            ],
        )?;
        if let FieldKind::Enum(values) = &field.kind {
            for (j, value) in values.iter().enumerate() {
                conn.execute(
                    "INSERT INTO schema_enum_values (schema, field, position, value)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![name.as_str(), field.name.as_str(), j as i64, value.as_str()],
                )?;
            }
        }
    }
    Ok(())
}

fn home_dir() -> Result<PathBuf, Error> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(Error::Fail(Fail::HomeNotSet))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_or_empty_is_not_equal() {
        assert!(!dec_eq(None, None).unwrap());
        assert!(!dec_eq(None, Some("1")).unwrap());
        assert!(!dec_eq(Some("1"), None).unwrap());
        assert!(!dec_eq(Some(""), Some("1")).unwrap());
        assert!(!dec_eq(Some("1"), Some("")).unwrap());
        assert!(!dec_eq(Some(""), Some("")).unwrap());
    }

    #[test]
    fn compares_decimal_values() {
        assert!(dec_eq(Some("39.6"), Some("39.60")).unwrap());
        assert!(!dec_eq(Some("39.6"), Some("39.61")).unwrap());
    }

    #[test]
    fn rejects_invalid_number() {
        assert!(dec_eq(Some("nope"), Some("1")).is_err());
    }

    #[test]
    fn sum_skips_null_and_empty() {
        assert_eq!(dec_add(Decimal::ZERO, None).unwrap(), Decimal::ZERO);
        assert_eq!(dec_add(Decimal::ZERO, Some("")).unwrap(), Decimal::ZERO);
        assert_eq!(
            dec_add("39.6".parse().unwrap(), Some("10"))
                .unwrap()
                .to_string(),
            "49.6"
        );
        assert!(dec_add(Decimal::MAX, Some("1")).is_err());
    }

    #[test]
    fn stamps_user_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bottle.db");
        let _db = Db::open(&path, TimeZone::UTC).unwrap();
        let conn = Sqlite::open(&path).unwrap();
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(v, USER_VERSION);
    }

    #[test]
    fn rejects_newer_user_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bottle.db");
        {
            let conn = Sqlite::open(&path).unwrap();
            conn.pragma_update(None, "user_version", USER_VERSION + 1)
                .unwrap();
        }
        let err = match Db::open(&path, TimeZone::UTC) {
            Ok(_) => panic!("expected unsupported store version"),
            Err(err) => err,
        };
        assert_eq!(
            err.to_string(),
            format!("unsupported store version: {}", USER_VERSION + 1)
        );
    }

    #[test]
    fn migrates_yaml_spec_column() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bottle.db");
        {
            let conn = Sqlite::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE schemas (
                    name TEXT PRIMARY KEY,
                    spec TEXT NOT NULL,
                    retired INTEGER NOT NULL DEFAULT 0
                 );",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO schemas (name, spec, retired) VALUES (?1, ?2, 0)",
                rusqlite::params![
                    "nutrition.meal",
                    "fields:\n  - name: kcal\n    type: number\n    required: true\n  - name: when\n    type: enum\n    required: true\n    values: [breakfast, lunch]\n"
                ],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
        }
        let db = Db::open(&path, TimeZone::UTC).unwrap();
        let schema =
            crate::store::load_schema(&db, &SchemaName::parse("nutrition.meal").unwrap()).unwrap();
        assert_eq!(schema.spec.fields.len(), 2);
        assert!(matches!(schema.spec.fields[0].kind, FieldKind::Number));
        let FieldKind::Enum(values) = &schema.spec.fields[1].kind else {
            panic!("expected enum");
        };
        assert_eq!(values[0].as_str(), "breakfast");
        assert_eq!(values[1].as_str(), "lunch");
        let v: i32 = db
            .as_ref()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(v, USER_VERSION);
        let mut info = db.as_ref().prepare("PRAGMA table_info(schemas)").unwrap();
        let names: Vec<String> = info
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(!names.iter().any(|n| n == "spec"), "{names:?}");
    }
}
