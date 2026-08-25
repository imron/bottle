use bottle::{Cmd, cmd, run};

use crate::common::{self, harness};

#[test]
fn overview_is_prose_not_tsv() {
    let out = run(None, None, Cmd::Help(cmd::Help { topic: None })).unwrap();
    assert!(out.starts_with("# overview\n"));
    assert!(out.contains("bottle is a store for events"));
    assert!(!out.contains('\t'));
}

#[test]
fn log_page() {
    let out = run(
        None,
        None,
        Cmd::Help(cmd::Help {
            topic: Some("log".into()),
        }),
    )
    .unwrap();
    assert!(out.starts_with("# log\n"));
    assert!(out.contains("write one entry"));
}

#[test]
fn schema_add_page() {
    let out = run(
        None,
        None,
        Cmd::Help(cmd::Help {
            topic: Some("schema add".into()),
        }),
    )
    .unwrap();
    assert!(out.starts_with("# schema add\n"));
}

#[test]
fn unknown_topic_is_usage() {
    let err = run(
        None,
        None,
        Cmd::Help(cmd::Help {
            topic: Some("nope".into()),
        }),
    )
    .unwrap_err();
    common::assert_usage(err, "unknown help topic");
}

#[test]
fn help_does_not_need_a_db() {
    run(None, None, Cmd::Help(cmd::Help { topic: None })).unwrap();
}

#[test]
fn other_commands_need_a_db() {
    let err = run(None, None, Cmd::SchemaList).unwrap_err();
    common::assert_fail(err, "db path required");
}

#[test]
fn run_opens_db_when_path_given() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let out = run(Some(&db), None, Cmd::SchemaList).unwrap();
    assert_eq!(out, "name\tretired\n");
}

#[test]
fn execute_serves_help() {
    let mut h = harness();
    let out = h.run_ok(Cmd::Help(cmd::Help { topic: None }));
    assert!(out.starts_with("# overview\n"));
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
        let out = run(
            None,
            None,
            Cmd::Help(cmd::Help {
                topic: Some(topic.into()),
            }),
        )
        .unwrap_or_else(|e| panic!("{topic}: {e}"));
        assert!(out.starts_with(&format!("# {topic}\n")), "{topic}: {out:?}");
    }
}
