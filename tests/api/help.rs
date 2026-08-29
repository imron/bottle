use bottle::{Bottle, Cmd, Style, cmd, execute, help, parse};
use jiff::tz::TimeZone;

use crate::common::{self, MEAL};

fn request(cmd: Cmd) -> bottle::Op {
    parse(cmd, &TimeZone::UTC).unwrap()
}

fn request_tz(cmd: Cmd, tz: &str) -> bottle::Op {
    parse(cmd, &TimeZone::get(tz).unwrap()).unwrap()
}

#[test]
fn overview_is_prose_not_tsv() {
    let out = help(None).unwrap();
    assert!(out.starts_with("# overview\n"));
    assert!(out.contains("bottle is a store for events"));
    assert!(!out.contains('\t'));
}

#[test]
fn log_page() {
    let out = help(Some("log")).unwrap();
    assert!(out.starts_with("# log\n"));
    assert!(out.contains("write one entry"));
}

#[test]
fn schema_add_page() {
    let out = help(Some("schema add")).unwrap();
    assert!(out.starts_with("# schema add\n"));
}

#[test]
fn unknown_topic_is_usage() {
    let err = help(Some("nope")).unwrap_err();
    common::assert_usage(err, "unknown help topic");
}

#[test]
fn help_does_not_need_a_db() {
    help(None).unwrap();
}

#[test]
fn open_creates_an_empty_db() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let mut bottle = Bottle::open(&db, None, None).unwrap();
    let out = execute(
        &mut bottle,
        request(Cmd::Schema(cmd::SchemaCmd::List)),
        Style::Tsv,
    )
    .unwrap();
    assert_eq!(out, "name\tretired\n");
}

#[test]
fn execute_uses_the_open_timezone() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let file = dir.path().join("meal.yaml");
    std::fs::write(&file, MEAL).unwrap();
    let mut bottle = Bottle::open(&db, Some("test".into()), Some(common::TZ)).unwrap();
    execute(
        &mut bottle,
        request_tz(
            Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
                name: "nutrition.meal".into(),
                file,
            })),
            common::TZ,
        ),
        Style::Tsv,
    )
    .unwrap();
    execute(
        &mut bottle,
        request_tz(
            Cmd::Log(cmd::Log {
                schema: "nutrition.meal".into(),
                at: Some("2026-08-22T08:14:00".into()),
                agent: None,
                links: vec![],
                fields: vec![
                    ("when".into(), "breakfast".into()),
                    ("what".into(), "eggs".into()),
                    ("kcal".into(), "1".into()),
                    ("protein".into(), "1".into()),
                    ("carbs".into(), "0".into()),
                ],
            }),
            common::TZ,
        ),
        Style::Tsv,
    )
    .unwrap();
    let out = execute(
        &mut bottle,
        request_tz(
            Cmd::Ls(cmd::Ls {
                filters: cmd::Filters {
                    schema: "nutrition.meal".into(),
                    agent: None,
                    wheres: vec![],
                    links: vec![],
                },
                from: None,
                to: None,
                include_ignored: false,
            }),
            common::TZ,
        ),
        Style::Tsv,
    )
    .unwrap();
    assert!(out.contains("2026-08-22T08:14:00+10:00"), "{out}");
}

#[test]
fn all_topics() {
    for topic in [
        "overview",
        "help",
        "schema",
        "schema list",
        "schema show",
        "schema add",
        "schema add-field",
        "schema add-value",
        "schema retire",
        "schema drop",
        "log",
        "ls",
        "get",
        "sum",
        "last",
        "today",
        "amend",
        "ignore",
        "unignore",
        "mcp",
    ] {
        let out = help(Some(topic)).unwrap_or_else(|e| panic!("{topic}: {e}"));
        assert!(out.starts_with(&format!("# {topic}\n")), "{topic}: {out:?}");
    }
}
