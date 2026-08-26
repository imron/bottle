use bottle::{Cmd, cmd};

use crate::common::{SESSION, SET, assert_fail, assert_usage, harness, seed_meals, tsv_lines};

#[test]
fn amend_and_unlink() {
    let mut h = harness();
    h.add_schema("fitness.session", SESSION);
    h.add_schema("fitness.set", SET);
    h.log(
        "fitness.session",
        &[("title", "upper")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    h.log(
        "fitness.set",
        &[("movement", "squat"), ("reps", "8")],
        &[("session", "fitness.session/1")],
        Some("2026-08-22T08:01:00Z"),
    );
    let out = h.run_ok(Cmd::Amend(cmd::Amend {
        schema: "fitness.set".into(),
        id: 1,
        at: None,
        agent: None,
        links: vec![],
        unlinks: vec!["session".into()],
        fields: vec![("reps".into(), "6".into())],
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines[1][2], "");
    h.run_ok(Cmd::Amend(cmd::Amend {
        schema: "fitness.set".into(),
        id: 1,
        at: None,
        agent: None,
        links: vec![],
        unlinks: vec!["session".into()],
        fields: vec![],
    }));
}

#[test]
fn amend_link_unlink_conflict() {
    let mut h = harness();
    h.add_schema("fitness.set", SET);
    h.log(
        "fitness.set",
        &[("movement", "squat"), ("reps", "8")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    let err = h
        .run(Cmd::Amend(cmd::Amend {
            schema: "fitness.set".into(),
            id: 1,
            at: None,
            agent: None,
            links: vec![("session".into(), "fitness.session/1".into())],
            unlinks: vec!["session".into()],
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "cannot link and unlink");
}

#[test]
fn amend_empty_and_missing() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Amend(cmd::Amend {
            schema: "nutrition.meal".into(),
            id: 1,
            at: None,
            agent: None,
            links: vec![],
            unlinks: vec![],
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "amend requires");
    let err = h
        .run(Cmd::Amend(cmd::Amend {
            schema: "nutrition.meal".into(),
            id: 99,
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            unlinks: vec![],
            fields: vec![],
        }))
        .unwrap_err();
    assert_fail(err, "not found");
}

#[test]
fn duplicate_unlink() {
    let mut h = harness();
    h.add_schema("fitness.set", SET);
    h.log(
        "fitness.set",
        &[("movement", "squat"), ("reps", "8")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    let err = h
        .run(Cmd::Amend(cmd::Amend {
            schema: "fitness.set".into(),
            id: 1,
            at: None,
            agent: None,
            links: vec![],
            unlinks: vec!["session".into(), "session".into()],
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "duplicate unlink");
}

#[test]
fn amend_at_agent_and_link() {
    let mut h = harness();
    h.add_schema("fitness.session", SESSION);
    h.add_schema("fitness.set", SET);
    h.log(
        "fitness.session",
        &[("title", "upper")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    h.log(
        "fitness.session",
        &[("title", "lower")],
        &[],
        Some("2026-08-22T09:00:00Z"),
    );
    h.log(
        "fitness.set",
        &[("movement", "squat"), ("reps", "8")],
        &[("session", "fitness.session/1")],
        Some("2026-08-22T08:01:00Z"),
    );
    let out = h.run_ok(Cmd::Amend(cmd::Amend {
        schema: "fitness.set".into(),
        id: 1,
        at: Some("2026-08-22T08:05:00Z".into()),
        agent: Some("coach".into()),
        links: vec![("session".into(), "fitness.session/2".into())],
        unlinks: vec![],
        fields: vec![("load".into(), "".into()), ("reps".into(), "6".into())],
    }));
    assert!(out.contains("fitness.session/2"));
}
