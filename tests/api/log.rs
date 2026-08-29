use bottle::{Bottle, Cmd, cmd};

use crate::common::{self, MEAL, SESSION, SET, assert_fail, assert_usage, harness, tsv_lines};

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
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert!(ls.contains("breakfast"));
    assert!(!ls.contains("Breakfast"));
}

#[test]
fn date_only_at_is_a_day() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let out = h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "eggs"),
            ("kcal", "1"),
            ("protein", "1"),
            ("carbs", "0"),
        ],
        &[],
        Some("2026-08-22"),
    );
    let lines = tsv_lines(&out);
    assert_eq!(lines[0], vec!["id", "at", "links"]);
    assert_eq!(lines[1][1], "2026-08-22");
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert!(ls.contains("2026-08-22\t"), "{ls}");
    assert!(!ls.contains("2026-08-22T"), "{ls}");
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
            file: None,
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
            file: None,
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
            file: None,
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
            file: None,
            fields: vec![
                ("when".into(), "breakfast".into()),
                ("what".into(), "eggs".into()),
                ("kcal".into(), "1e3".into()),
                ("protein".into(), "1".into()),
                ("carbs".into(), "0".into()),
            ],
        }))
        .unwrap_err();
    assert_fail(err, "invalid number: 1e3");
}

#[test]
fn non_canonical_number_rejected() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    for raw in ["01", "+1", "1.", ".5"] {
        let err = h
            .run(Cmd::Log(cmd::Log {
                schema: "nutrition.meal".into(),
                at: Some("2026-08-22T08:14:00Z".into()),
                agent: None,
                links: vec![],
                file: None,
                fields: {
                    let mut f = meal_fields();
                    f[2] = ("kcal".into(), raw.into());
                    f
                },
            }))
            .unwrap_err();
        assert_fail(err, &format!("invalid number: {raw}"));
    }
}

#[test]
fn agent_from_open() {
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
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&ls);
    let agent_idx = lines[0].iter().position(|c| *c == "agent").unwrap();
    assert_eq!(lines[1][agent_idx], "test");
}

#[test]
fn agent_rejects_empty() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: Some("   ".into()),
            links: vec![],
            file: None,
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_fail(err, "empty agent");
}

#[test]
fn agent_rejects_tab() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: Some("a\tb".into()),
            links: vec![],
            file: None,
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_fail(err, "agent may not contain tab or newline");
    let db = h.dir.path().join("bottle.db");
    let err = match Bottle::open(&db, Some("a\tb".into()), Some(common::TZ)) {
        Ok(_) => panic!("expected fail"),
        Err(e) => e,
    };
    assert_fail(err, "agent may not contain tab or newline");
}

#[test]
fn empty_open_agent_is_rejected() {
    let h = harness();
    let db = h.dir.path().join("other.db");
    for agent in ["", "   "] {
        let err = match Bottle::open(&db, Some(agent.into()), Some(common::TZ)) {
            Ok(_) => panic!("expected fail for {agent:?}"),
            Err(e) => e,
        };
        assert_fail(err, "empty agent");
    }
}

#[test]
fn agent_trims_spaces() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.run_ok(Cmd::Log(cmd::Log {
        schema: "nutrition.meal".into(),
        at: Some("2026-08-22T08:14:00Z".into()),
        agent: Some("  coach  ".into()),
        links: vec![],
        file: None,
        fields: meal_fields(),
    }));
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: Some(" coach".into()),
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&ls);
    let agent_idx = lines[0].iter().position(|c| *c == "agent").unwrap();
    assert_eq!(lines[1][agent_idx], "coach");
}

#[test]
fn unset_agent_defaults_to_bottle() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let db = h.dir.path().join("bottle.db");
    let mut bottle = Bottle::open(&db, None, Some(common::TZ)).unwrap();
    let request = bottle::parse(
        Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: Some("2026-08-22T08:14:00Z".into()),
            agent: None,
            links: vec![],
            file: None,
            fields: vec![
                ("when".into(), "breakfast".into()),
                ("what".into(), "eggs".into()),
                ("kcal".into(), "1".into()),
                ("protein".into(), "1".into()),
                ("carbs".into(), "0".into()),
            ],
        }),
        bottle.tz(),
    )
    .unwrap();
    bottle::execute(&mut bottle, request, bottle::Style::Tsv).unwrap();
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
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
            file: None,
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
            file: None,
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
            file: None,
            fields: meal_fields(),
        }))
        .unwrap_err();
    assert_fail(err, "collides with field");
    let err = h
        .run(Cmd::Ls(cmd::Ls {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![("what".into(), "nutrition.meal/1".into())],
                excludes: vec![],
            },
            from: None,
            to: None,
            include_ignored: false,
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
                file: None,
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
    for at in [
        "2026-08-22 08:14:00",
        "2026-08-22T08:14",
        "2026-08-22T08:14:00+1000",
        "2026-08-22T08:14+10",
    ] {
        let out = h.log(
            "nutrition.meal",
            &[
                ("when", "breakfast"),
                ("what", at),
                ("kcal", "1"),
                ("protein", "1"),
                ("carbs", "0"),
            ],
            &[],
            Some(at),
        );
        assert!(out.contains("2026-08-22T08:14:00+10:00"), "{at}: {out}");
    }
    for at in [
        "nope",
        "2026-08-22Tnotime1",
        "2026-08-22T08:14:00.5Z",
        "2026-08-22T08:14:00x",
        "2026-08-22T08:14:00.0+10:00",
        "2026-08-22T08:14:00z",
    ] {
        let err = h
            .run(Cmd::Log(cmd::Log {
                schema: "nutrition.meal".into(),
                at: Some(at.into()),
                agent: None,
                links: vec![],
                file: None,
                fields: meal_fields(),
            }))
            .unwrap_err();
        assert_usage(err, "invalid time");
    }
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
            file: None,
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
            file: None,
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
            file: None,
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
                file: None,
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
            file: None,
            fields: {
                let mut f = meal_fields();
                f[2] = ("kcal".into(), "nope".into());
                f
            },
        }))
        .unwrap_err();
    assert_fail(err, "invalid number: nope");
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
            file: None,
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
            file: None,
            fields: {
                let mut f = meal_fields();
                f[0] = ("when".into(), "not-a-meal".into());
                f
            },
        }))
        .unwrap_err();
    assert_fail(err, "invalid");
}

#[test]
fn link_target_must_exist() {
    let mut h = harness();
    h.add_schema("fitness.set", SET);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "fitness.set".into(),
            at: Some("2026-08-22T08:00:00Z".into()),
            agent: None,
            links: vec![("session".into(), "fitness.session/1".into())],
            file: None,
            fields: vec![
                ("movement".into(), "squat".into()),
                ("reps".into(), "8".into()),
            ],
        }))
        .unwrap_err();
    assert_fail(err, "unknown schema");
}

fn meal_log(when: &str, kcal: &str) -> cmd::Log {
    cmd::Log {
        schema: "nutrition.meal".into(),
        at: Some("2026-08-22T08:14:00Z".into()),
        agent: None,
        links: vec![],
        file: None,
        fields: vec![
            ("when".into(), when.into()),
            ("what".into(), "eggs".into()),
            ("kcal".into(), kcal.into()),
            ("protein".into(), "1".into()),
            ("carbs".into(), "0".into()),
        ],
    }
}

#[test]
fn log_entries_one_table() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let out = h
        .log_entries(vec![meal_log("breakfast", "1"), meal_log("lunch", "2")])
        .unwrap();
    let lines = tsv_lines(&out);
    assert_eq!(lines[0], vec!["id", "at", "links"]);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1][0], "1");
    assert_eq!(lines[2][0], "2");
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&ls).len(), 3);
}

#[test]
fn log_entries_rolls_back_on_error() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let mut bad = meal_log("lunch", "2");
    bad.fields.pop();
    let err = h
        .log_entries(vec![meal_log("breakfast", "1"), bad])
        .unwrap_err();
    assert_fail(err, "missing required");
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&ls).len(), 1);
}

#[test]
fn log_file_writes_all_rows() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let path = h.dir.path().join("meals.tsv");
    std::fs::write(
        &path,
        "at\twhen\twhat\tkcal\tprotein\tcarbs\n\
         2026-08-22T08:14:00Z\tbreakfast\teggs\t1\t1\t0\n\
         2026-08-22T12:00:00Z\tlunch\trice\t2\t2\t1\n",
    )
    .unwrap();
    let out = h.run_ok(Cmd::Log(cmd::Log {
        schema: "nutrition.meal".into(),
        at: None,
        agent: None,
        links: vec![],
        file: Some(path),
        fields: vec![],
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines[0], vec!["id", "at", "links"]);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1][0], "1");
    assert_eq!(lines[2][0], "2");
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&ls).len(), 3);
}

#[test]
fn log_file_rolls_back_on_error() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let path = h.dir.path().join("meals.tsv");
    std::fs::write(
        &path,
        "when\twhat\tkcal\tprotein\tcarbs\n\
         breakfast\teggs\t1\t1\t0\n\
         lunch\trice\t2\t2\t\n",
    )
    .unwrap();
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(path),
            fields: vec![],
        }))
        .unwrap_err();
    assert_fail(err, "missing required");
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&ls).len(), 1);
}

#[test]
fn log_file_needs_header() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let empty = h.dir.path().join("empty.tsv");
    std::fs::write(&empty, "").unwrap();
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(empty),
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "header");
    let header_only = h.dir.path().join("header.tsv");
    std::fs::write(&header_only, "when\twhat\tkcal\tprotein\tcarbs\n").unwrap();
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(header_only),
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "at least one entry");
}

#[test]
fn log_file_uses_command_defaults() {
    let mut h = harness();
    h.add_schema("fitness.session", SESSION);
    h.add_schema("fitness.set", SET);
    h.log(
        "fitness.session",
        &[("title", "upper")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    let path = h.dir.path().join("sets.tsv");
    std::fs::write(&path, "movement\treps\nsquat\t8\nbench\t5\n").unwrap();
    let out = h.run_ok(Cmd::Log(cmd::Log {
        schema: "fitness.set".into(),
        at: Some("2026-08-22T08:01:00Z".into()),
        agent: Some("coach".into()),
        links: vec![("session".into(), "fitness.session/1".into())],
        file: Some(path),
        fields: vec![],
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 3);
    assert!(lines[1][2].contains("session=fitness.session/1"), "{out}");
    assert!(lines[2][2].contains("session=fitness.session/1"), "{out}");
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "fitness.set".into(),
            agent: Some("coach".into()),
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&ls).len(), 3);
}

#[test]
fn log_file_links_and_missing_at() {
    let mut h = harness();
    h.add_schema("fitness.session", SESSION);
    h.add_schema("fitness.set", SET);
    h.log(
        "fitness.session",
        &[("title", "upper")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    let path = h.dir.path().join("sets.tsv");
    std::fs::write(
        &path,
        "links\tmovement\treps\n\
         session=fitness.session/1\tsquat\t8\n",
    )
    .unwrap();
    let out = h.run_ok(Cmd::Log(cmd::Log {
        schema: "fitness.set".into(),
        at: None,
        agent: None,
        links: vec![],
        file: Some(path),
        fields: vec![],
    }));
    assert!(out.contains("session=fitness.session/1"), "{out}");
}

#[test]
fn log_file_missing_is_fail() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(h.dir.path().join("nope.tsv")),
            fields: vec![],
        }))
        .unwrap_err();
    assert_fail(err, "file not found");
}

#[test]
fn log_file_wrong_width_and_duplicate_header() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let wide = h.dir.path().join("wide.tsv");
    std::fs::write(&wide, "when\twhat\nbreakfast\teggs\textra\n").unwrap();
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(wide),
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "wrong number of columns");
    let dup = h.dir.path().join("dup.tsv");
    std::fs::write(&dup, "when\twhen\nbreakfast\tlunch\n").unwrap();
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(dup),
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "duplicate column");
    let short = h.dir.path().join("short.tsv");
    std::fs::write(&short, "when\twhat\nbreakfast\n").unwrap();
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(short),
            fields: vec![],
        }))
        .unwrap_err();
    assert_usage(err, "wrong number of columns");
}

#[test]
fn log_file_tsv_cells_win_over_command() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let path = h.dir.path().join("meals.tsv");
    std::fs::write(
        &path,
        "at\tagent\twhen\twhat\tkcal\tprotein\tcarbs\n\
         2026-08-22T08:14:00Z\tfilebot\tbreakfast\teggs\t1\t1\t0\n",
    )
    .unwrap();
    h.run_ok(Cmd::Log(cmd::Log {
        schema: "nutrition.meal".into(),
        at: Some("2026-08-22T12:00:00Z".into()),
        agent: Some("cli".into()),
        links: vec![],
        file: Some(path),
        fields: vec![("kcal".into(), "99".into())],
    }));
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: true,
    }));
    assert!(ls.contains("filebot"), "{ls}");
    assert!(!ls.contains("cli"), "{ls}");
    assert!(ls.contains("18:14:00"), "{ls}");
    assert!(!ls.contains("22:00:00"), "{ls}");
    assert!(!ls.contains("99"), "{ls}");
}

#[test]
fn log_file_command_fields_fill_missing_columns() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let path = h.dir.path().join("meals.tsv");
    std::fs::write(
        &path,
        "when\twhat\tprotein\tcarbs\n\
         breakfast\teggs\t1\t0\n",
    )
    .unwrap();
    h.run_ok(Cmd::Log(cmd::Log {
        schema: "nutrition.meal".into(),
        at: Some("2026-08-22T08:14:00Z".into()),
        agent: None,
        links: vec![],
        file: Some(path),
        fields: vec![("kcal".into(), "10".into())],
    }));
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&ls);
    let kcal = lines[0].iter().position(|c| *c == "kcal").unwrap();
    assert_eq!(lines[1][kcal], "10");
}

#[test]
fn log_file_link_overrides_command_link() {
    let mut h = harness();
    h.add_schema("fitness.session", SESSION);
    h.add_schema("fitness.set", SET);
    h.log(
        "fitness.session",
        &[("title", "one")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    h.log(
        "fitness.session",
        &[("title", "two")],
        &[],
        Some("2026-08-22T09:00:00Z"),
    );
    let path = h.dir.path().join("sets.tsv");
    std::fs::write(
        &path,
        "links\tmovement\treps\n\
         session=fitness.session/2\tsquat\t8\n",
    )
    .unwrap();
    let out = h.run_ok(Cmd::Log(cmd::Log {
        schema: "fitness.set".into(),
        at: Some("2026-08-22T08:01:00Z".into()),
        agent: None,
        links: vec![("session".into(), "fitness.session/1".into())],
        file: Some(path),
        fields: vec![],
    }));
    assert!(out.contains("session=fitness.session/2"), "{out}");
    assert!(!out.contains("session=fitness.session/1"), "{out}");
}

#[test]
fn log_file_reserved_header_and_directory() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let reserved = h.dir.path().join("id.tsv");
    std::fs::write(
        &reserved,
        "id\twhen\twhat\tkcal\tprotein\tcarbs\n\
         1\tbreakfast\teggs\t1\t1\t0\n",
    )
    .unwrap();
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(reserved),
            fields: vec![],
        }))
        .unwrap_err();
    assert_fail(err, "reserved field name");
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
            agent: None,
            links: vec![],
            file: Some(h.dir.path().to_path_buf()),
            fields: vec![],
        }))
        .unwrap_err();
    assert_fail(err, "io error");
}
