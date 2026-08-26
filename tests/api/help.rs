use bottle::{Cmd, cmd, help, run};

use crate::common::{self, MEAL};

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
fn other_commands_need_a_db() {
    let err = run(None, None, None, Cmd::Schema(cmd::SchemaCmd::List)).unwrap_err();
    common::assert_fail(err, "db path required");
}

#[test]
fn run_opens_db_when_path_given() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let out = run(Some(&db), None, None, Cmd::Schema(cmd::SchemaCmd::List)).unwrap();
    assert_eq!(out, "name\tretired\n");
}

#[test]
fn run_uses_given_timezone() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let file = dir.path().join("meal.yaml");
    std::fs::write(&file, MEAL).unwrap();
    run(
        Some(&db),
        Some("test".into()),
        Some(common::TZ),
        Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
            name: "nutrition.meal".into(),
            file,
        })),
    )
    .unwrap();
    run(
        Some(&db),
        Some("test".into()),
        Some(common::TZ),
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
    )
    .unwrap();
    let out = run(
        Some(&db),
        None,
        Some(common::TZ),
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
        "mcp",
    ] {
        let out = help(Some(topic)).unwrap_or_else(|e| panic!("{topic}: {e}"));
        assert!(out.starts_with(&format!("# {topic}\n")), "{topic}: {out:?}");
    }
}
