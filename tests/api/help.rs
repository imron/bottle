use bottle::{Cmd, run};

use crate::common;

#[test]
fn overview_is_prose_not_tsv() {
    let out = run(None, None, Cmd::Help { topic: None }).unwrap();
    assert!(out.starts_with("## overview\n"));
    assert!(out.contains("bottle is a store for events"));
    assert!(!out.contains('\t'));
}

#[test]
fn log_page() {
    let out = run(
        None,
        None,
        Cmd::Help {
            topic: Some("log".into()),
        },
    )
    .unwrap();
    assert!(out.starts_with("## log\n"));
    assert!(out.contains("Writes one entry"));
}

#[test]
fn schema_add_page() {
    let out = run(
        None,
        None,
        Cmd::Help {
            topic: Some("schema add".into()),
        },
    )
    .unwrap();
    assert!(out.starts_with("## schema add\n"));
}

#[test]
fn unknown_topic_is_usage() {
    let err = run(
        None,
        None,
        Cmd::Help {
            topic: Some("nope".into()),
        },
    )
    .unwrap_err();
    common::assert_usage(err, "unknown help topic");
}

#[test]
fn help_does_not_need_a_db() {
    run(None, None, Cmd::Help { topic: None }).unwrap();
}
