use bottle::{Cmd, cmd};

use crate::common::{
    MEAL, SESSION, SET, assert_fail, assert_usage, harness, seed_meals, tsv_lines,
};

#[test]
fn ls_columns_and_number_format() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&out);
    assert_eq!(
        lines[0],
        vec![
            "id", "at", "links", "when", "what", "kcal", "protein", "carbs", "fat", "agent"
        ]
    );
    assert_eq!(lines[1][5], "568");
    assert_eq!(lines[1][6], "49");
    assert_eq!(lines[1][8], "39.6");
    assert_eq!(lines.len(), 3);
}

#[test]
fn ls_keeps_logged_number_scale() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "eggs"),
            ("kcal", "568"),
            ("protein", "49"),
            ("carbs", "5"),
            ("fat", "39.60"),
        ],
        &[],
        Some("2026-08-22T08:14:00Z"),
    );
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines[1][8], "39.60");
    let matched = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![("fat".into(), "39.6".into())],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&matched).len(), 2);
}

#[test]
fn ls_from_instant() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: Some("2026-08-23T12:00:00Z".into()),
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][4], "rice");
}

#[test]
fn ls_instant_to_is_inclusive() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: Some("2026-08-22T08:14:00Z".into()),
        include_ignored: false,
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][4], "eggs");
}

#[test]
fn ls_from_to_dates() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: Some("2026-08-22".into()),
        to: Some("2026-08-22".into()),
        include_ignored: false,
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][4], "eggs");
}

#[test]
fn ls_where_enum_folds() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![("when".into(), "LUNCH".into())],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][3], "lunch");
}

#[test]
fn ls_where_text_is_exact() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![("what".into(), "Eggs".into())],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&out).len(), 1);
}

#[test]
fn ls_where_agent_reserved() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Ls(cmd::Ls {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![("agent".into(), "test".into())],
                links: vec![],
            },
            from: None,
            to: None,
            include_ignored: false,
        }))
        .unwrap_err();
    assert_usage(err, "reserved");
}

#[test]
fn ls_filter_agent_flag() {
    let mut h = harness();
    seed_meals(&mut h);
    let none = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: Some("other".into()),
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&none).len(), 1);
    let some = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: Some("test".into()),
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&some).len(), 3);
}

#[test]
fn ls_include_ignored() {
    let mut h = harness();
    seed_meals(&mut h);
    let before = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: true,
    }));
    let before_lines = tsv_lines(&before);
    assert_eq!(before_lines[0].last().copied(), Some("ignored"));
    assert_eq!(before_lines.len(), 3);
    assert_eq!(before_lines[1].last().copied(), Some("false"));
    assert_eq!(before_lines[2].last().copied(), Some("false"));
    h.run_ok(Cmd::Ignore(cmd::Ignore {
        schema: "nutrition.meal".into(),
        id: 1,
    }));
    let hidden = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&hidden).len(), 2);
    assert!(!tsv_lines(&hidden)[0].contains(&"ignored"));
    let shown = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: true,
    }));
    let shown_lines = tsv_lines(&shown);
    assert_eq!(shown_lines.len(), 3);
    assert_eq!(shown_lines[0].last().copied(), Some("ignored"));
    assert_eq!(shown_lines[1][0], "1");
    assert_eq!(shown_lines[1].last().copied(), Some("true"));
    assert_eq!(shown_lines[2][0], "2");
    assert_eq!(shown_lines[2].last().copied(), Some("false"));
}

#[test]
fn links_where_and_group() {
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
        &[("movement", "squat"), ("reps", "8"), ("load", "24")],
        &[("session", "fitness.session/1")],
        Some("2026-08-22T08:01:00Z"),
    );
    h.log(
        "fitness.set",
        &[("movement", "plank"), ("reps", "1")],
        &[],
        Some("2026-08-22T08:02:00Z"),
    );
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "fitness.set".into(),
            agent: None,
            wheres: vec![],
            links: vec![("session".into(), "fitness.session/1".into())],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&ls);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][2], "session=fitness.session/1");
    let grouped = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "fitness.set".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        field: "reps".into(),
        from: None,
        to: None,
        group: Some("session".into()),
    }));
    assert!(grouped.contains("fitness.session/1\t8"));
}

#[test]
fn ls_where_number_skips_null() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![("fat".into(), "39.6".into())],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][4], "eggs");
}

#[test]
fn invalid_date_bound() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Ls(cmd::Ls {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![],
            },
            from: Some("2026-13-01".into()),
            to: None,
            include_ignored: false,
        }))
        .unwrap_err();
    assert_usage(err, "invalid date");
}

#[test]
fn where_invalid_ident() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Ls(cmd::Ls {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![("When".into(), "breakfast".into())],
                links: vec![],
            },
            from: None,
            to: None,
            include_ignored: false,
        }))
        .unwrap_err();
    assert_fail(err, "invalid field name");
}

#[test]
fn where_empty_value_is_usage() {
    let mut h = harness();
    seed_meals(&mut h);
    for field in ["what", "fat", "when"] {
        let err = h
            .run(Cmd::Ls(cmd::Ls {
                filters: cmd::Filters {
                    schema: "nutrition.meal".into(),
                    agent: None,
                    wheres: vec![(field.into(), String::new())],
                    links: vec![],
                },
                from: None,
                to: None,
                include_ignored: false,
            }))
            .unwrap_err();
        assert_usage(err, &format!("empty {field}"));
    }
}

#[test]
fn where_unknown_field() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Ls(cmd::Ls {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![("nope".into(), "x".into())],
                links: vec![],
            },
            from: None,
            to: None,
            include_ignored: false,
        }))
        .unwrap_err();
    assert_fail(err, "unknown field");
}
