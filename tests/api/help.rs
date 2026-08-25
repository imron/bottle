use bottle::{Cmd, cmd, run};

use crate::common;

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
fn bottle_db_env_sets_default_path() {
    unsafe {
        std::env::set_var("BOTTLE_DB", "/tmp/bottle-cov.db");
    }
    let path = bottle::default_db_path().unwrap();
    unsafe {
        std::env::remove_var("BOTTLE_DB");
    }
    assert_eq!(path, std::path::PathBuf::from("/tmp/bottle-cov.db"));
}
