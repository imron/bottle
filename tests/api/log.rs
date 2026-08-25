use bottle::{Bottle, Cmd, cmd};

use crate::common::{MEAL, SESSION, SET, assert_fail, assert_usage, harness, tsv_lines};

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

fn meal_fields() -> Vec<(String, String)> {
    vec![
        ("when".into(), "breakfast".into()),
        ("what".into(), "eggs".into()),
        ("kcal".into(), "1".into()),
        ("protein".into(), "1".into()),
        ("carbs".into(), "0".into()),
    ]
}

#[test]
fn duplicate_field_and_link_name() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: {
                let mut f = meal_fields();
                f.push(("when".into(), "lunch".into()));
                f
            },
        }))
        .unwrap_err();
    assert_usage(err, "duplicate field");
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
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T09:00:00Z".into()),
            agent: None,
            links: vec![
                ("ref".into(), "nutrition.meal/1".into()),
                ("ref".into(), "nutrition.meal/1".into()),
            ],
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_usage(err, "duplicate link name");
}

#[test]
fn link_name_collides_with_field() {
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
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T09:00:00Z".into()),
            agent: None,
            links: vec![("what".into(), "nutrition.meal/1".into())],
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_fail(err, "collides with field");
}

#[test]
fn invalid_link_targets() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    for target in ["noshift", "nutrition.meal/0", "nutrition.meal/x"] {
        let err = h
            .run(Cmd::Log(cmd::Log {
                schema: "nutrition.meal".into(),
                at: Some("2026-08-22T08:14:00Z".into()),
                agent: None,
                links: vec![("ref".into(), target.into())],
                fields: meal_fields(),
            }))
            .unwrap_err();
        assert_usage(err, "invalid link target");
    }
}

#[test]
fn time_parse_variants() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22 08:14:00".into()),
            agent: None,
            links: vec![],
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_usage(err, "time must use T");
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00+1000".into()),
            agent: None,
            links: vec![],
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_usage(err, "colon");
    for at in [
        "nope",
        "2026-08-22Tnotime1",
        "2026-08-22T08:14:00.5Z",
        "2026-08-22T08:14:00x",
        "2026-08-22T08:14:00.0+10:00",
    ] {
        let err = h
            .run(Cmd::Log(cmd::Log {
                schema: "nutrition.meal".into(),
                at: Some(at.into()),
                agent: None,
                links: vec![],
                fields: meal_fields(),
            }))
            .unwrap_err();
        assert_usage(err, "invalid time");
    }
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "local"),
            ("kcal", "1"),
            ("protein", "1"),
            ("carbs", "0"),
        ],
        &[],
        Some("2026-08-22T08:14:00"),
    );
}

#[test]
fn empty_required_field() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: {
                let mut f = meal_fields();
                f[1] = ("what".into(), "".into());
                f
            },
        }))
        .unwrap_err();
    assert_fail(err, "missing required");
}

#[test]
fn link_target_missing_entry() {
    let mut h = harness();
    h.add_schema("fitness.session", SESSION);
    h.add_schema("fitness.set", SET);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "fitness.set".into(),
            at: Some("2026-08-22T08:00:00Z".into()),
            agent: None,
            links: vec![("session".into(), "fitness.session/99".into())],
            fields: vec![
                ("movement".into(), "squat".into()),
                ("reps".into(), "8".into()),
            ],
        }))
        .unwrap_err();
    assert_fail(err, "link target missing");
}

#[test]
fn bad_link_names() {
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
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T09:00:00Z".into()),
            agent: None,
            links: vec![("Ref".into(), "nutrition.meal/1".into())],
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_fail(err, "invalid link name");
    for name in ["id", "day"] {
        let err = h
            .run(Cmd::Log(cmd::Log {
                schema: "nutrition.meal".into(),
                at: Some("2026-08-22T09:00:00Z".into()),
                agent: None,
                links: vec![(name.into(), "nutrition.meal/1".into())],
                fields: meal_fields(),
            }))
            .unwrap_err();
        assert_fail(err, "reserved link name");
    }
}

#[test]
fn invalid_plain_number() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: {
                let mut f = meal_fields();
                f[2] = ("kcal".into(), "nope".into());
                f
            },
        }))
        .unwrap_err();
    assert_fail(err, "invalid number");
}

#[test]
fn newline_and_bad_enum() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: {
                let mut f = meal_fields();
                f[1] = ("what".into(), "a\nb".into());
                f
            },
        }))
        .unwrap_err();
    assert_fail(err, "tab or newline");
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            fields: {
                let mut f = meal_fields();
                f[0] = ("when".into(), "not-a-meal".into());
                f
            },
        }))
        .unwrap_err();
    assert_fail(err, "invalid");
}
