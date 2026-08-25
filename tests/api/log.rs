use bottle::{Bottle, Cmd, cmd};

use crate::common::{MEAL, assert_fail, assert_usage, harness, tsv_lines};

#[test]
fn log_prints_id_at_links() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let out = h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "4 eggs"),
            ("kcal", "568"),
            ("protein", "49"),
            ("carbs", "5"),
            ("fat", "39.6"),
        ],
        &[],
        Some("2026-08-22T08:14:00+10:00"),
    );
    let lines = tsv_lines(&out);
    assert_eq!(lines[0], vec!["id", "at", "links"]);
    assert_eq!(lines[1][0], "1");
    assert_eq!(lines[1][1], "2026-08-22T08:14:00+10:00");
    assert_eq!(lines[1][2], "");
}

#[test]
fn enum_folds_on_write() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.log(
        "nutrition.meal",
        &[
            ("when", "Breakfast"),
            ("what", "eggs"),
            ("kcal", "1"),
            ("protein", "1"),
            ("carbs", "0"),
        ],
        &[],
        Some("2026-08-22T08:14:00Z"),
    );
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        include_ignored: false,
    }));
    assert!(ls.contains("breakfast"));
    assert!(!ls.contains("Breakfast"));
}

#[test]
fn date_only_at_is_usage() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22".into()),
            agent: None,
            links: vec![],
            fields: vec![
                ("when".into(), "breakfast".into()),
                ("what".into(), "eggs".into()),
                ("kcal".into(), "1".into()),
                ("protein".into(), "1".into()),
                ("carbs".into(), "0".into()),
            ],
        }))
        .unwrap_err();
    assert_usage(err, "date-only");
}

#[test]
fn unknown_field_rejected() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: vec![
                ("when".into(), "breakfast".into()),
                ("what".into(), "eggs".into()),
                ("kcal".into(), "1".into()),
                ("protein".into(), "1".into()),
                ("carbs".into(), "0".into()),
                ("nope".into(), "x".into()),
            ],
        }))
        .unwrap_err();
    assert_fail(err, "unknown field");
}

#[test]
fn missing_required_rejected() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: vec![("when".into(), "breakfast".into())],
        }))
        .unwrap_err();
    assert_fail(err, "missing required");
}

#[test]
fn tab_in_text_rejected() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: vec![
                ("when".into(), "breakfast".into()),
                ("what".into(), "a\tb".into()),
                ("kcal".into(), "1".into()),
                ("protein".into(), "1".into()),
                ("carbs".into(), "0".into()),
            ],
        }))
        .unwrap_err();
    assert_fail(err, "tab");
}

#[test]
fn scientific_notation_rejected() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: vec![
                ("when".into(), "breakfast".into()),
                ("what".into(), "eggs".into()),
                ("kcal".into(), "1e3".into()),
                ("protein".into(), "1".into()),
                ("carbs".into(), "0".into()),
            ],
        }))
        .unwrap_err();
    assert_fail(err, "invalid number");
}

#[test]
fn default_agent() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "eggs"),
            ("kcal", "1"),
            ("protein", "1"),
            ("carbs", "0"),
        ],
        &[],
        Some("2026-08-22T08:14:00Z"),
    );
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        include_ignored: false,
    }));
    let lines = tsv_lines(&ls);
    let agent_idx = lines[0].iter().position(|c| *c == "agent").unwrap();
    assert_eq!(lines[1][agent_idx], "test");
}

#[test]
fn unset_agent_defaults_to_bottle() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let db = h.dir.path().join("bottle.db");
    let mut bottle = Bottle::open(&db, None).unwrap();
    bottle::execute(
        &mut bottle,
        Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
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
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        include_ignored: false,
    }));
    let lines = tsv_lines(&ls);
    let agent_idx = lines[0].iter().position(|c| *c == "agent").unwrap();
    assert_eq!(lines[1][agent_idx], "bottle");
}
