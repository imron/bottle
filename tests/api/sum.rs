use bottle::{Cmd, FieldType, cmd};

use crate::common::{SESSION, SET, assert_fail, harness, seed_meals, tsv_lines};

#[test]
fn sum_and_group_day() {
    let mut h = harness();
    seed_meals(&mut h);
    let total = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "protein".into(),
        from: None,
        to: None,
        group: None,
    }));
    assert_eq!(
        tsv_lines(&total),
        vec![vec!["field", "value"], vec!["protein", "59"]]
    );
    let empty = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "protein".into(),
        from: Some("2020-01-01".into()),
        to: Some("2020-01-02".into()),
        group: None,
    }));
    assert_eq!(
        tsv_lines(&empty),
        vec![vec!["field", "value"], vec!["protein", "0"]]
    );
    let grouped = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "protein".into(),
        from: None,
        to: None,
        group: Some("day".into()),
    }));
    assert!(grouped.starts_with("day\tvalue\n"));
}

#[test]
fn sum_skips_empty_number() {
    let mut h = harness();
    seed_meals(&mut h);
    let total = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "fat".into(),
        from: None,
        to: None,
        group: None,
    }));
    assert_eq!(
        tsv_lines(&total),
        vec![vec!["field", "value"], vec!["fat", "39.6"]]
    );
    let grouped = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "fat".into(),
        from: None,
        to: None,
        group: Some("day".into()),
    }));
    assert_eq!(
        tsv_lines(&grouped),
        vec![vec!["day", "value"], vec!["2026-08-22", "39.6"]]
    );
}

#[test]
fn sum_group_link_skips_empty_number() {
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
    let grouped = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "fitness.set".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "load".into(),
        from: None,
        to: None,
        group: Some("session".into()),
    }));
    assert_eq!(
        tsv_lines(&grouped),
        vec![vec!["session", "value"], vec!["fitness.session/1", "24"]]
    );
}

#[test]
fn sum_rejects_text_field() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Sum(cmd::Sum {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![],
                excludes: vec![],
            },
            field: "what".into(),
            from: None,
            to: None,
            group: None,
        }))
        .unwrap_err();
    assert_fail(err, "not a number");
}

#[test]
fn sum_group_week_month_year() {
    let mut h = harness();
    seed_meals(&mut h);
    for unit in ["week", "month", "year"] {
        let out = h.run_ok(Cmd::Sum(cmd::Sum {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![],
                excludes: vec![],
            },
            field: "protein".into(),
            from: None,
            to: None,
            group: Some(unit.into()),
        }));
        assert!(out.starts_with(&format!("{unit}\tvalue\n")), "{out}");
        assert!(out.contains("59"), "{out}");
    }
}

#[test]
fn sum_unknown_field() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Sum(cmd::Sum {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![],
                excludes: vec![],
            },
            field: "fiber".into(),
            from: None,
            to: None,
            group: None,
        }))
        .unwrap_err();
    assert_fail(err, "unknown field");
}

#[test]
fn sum_rejects_invalid_field_name() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Sum(cmd::Sum {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![],
                excludes: vec![],
            },
            field: "Protein".into(),
            from: None,
            to: None,
            group: None,
        }))
        .unwrap_err();
    assert_fail(err, "invalid field name");
}

#[test]
fn sum_group_enum_and_text_fields() {
    let mut h = harness();
    seed_meals(&mut h);
    let by_when = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "protein".into(),
        from: None,
        to: None,
        group: Some("when".into()),
    }));
    assert_eq!(
        tsv_lines(&by_when),
        vec![
            vec!["when", "value"],
            vec!["breakfast", "49"],
            vec!["lunch", "10"],
        ]
    );
    let by_what = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "protein".into(),
        from: None,
        to: None,
        group: Some("what".into()),
    }));
    assert_eq!(
        tsv_lines(&by_what),
        vec![
            vec!["what", "value"],
            vec!["eggs", "49"],
            vec!["rice", "10"],
        ]
    );
}

#[test]
fn sum_group_rejects_number_field() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Sum(cmd::Sum {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![],
                excludes: vec![],
            },
            field: "protein".into(),
            from: None,
            to: None,
            group: Some("kcal".into()),
        }))
        .unwrap_err();
    assert_fail(err, "cannot group by number field: kcal");
}

#[test]
fn sum_group_empty_optional_field() {
    let mut h = harness();
    seed_meals(&mut h);
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
        schema: "nutrition.meal".into(),
        name: "tag".into(),
        type_: FieldType::Text,
        values: None,
        default: None,
    })));
    h.log(
        "nutrition.meal",
        &[
            ("when", "snack"),
            ("what", "nuts"),
            ("kcal", "1"),
            ("protein", "5"),
            ("carbs", "1"),
            ("tag", "work"),
        ],
        &[],
        Some("2026-08-24T08:00:00Z"),
    );
    let grouped = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "protein".into(),
        from: None,
        to: None,
        group: Some("tag".into()),
    }));
    assert_eq!(
        tsv_lines(&grouped),
        vec![vec!["tag", "value"], vec!["", "59"], vec!["work", "5"],]
    );
}

#[test]
fn sum_exclude_drops_matching_rows() {
    let mut h = harness();
    seed_meals(&mut h);
    let total = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![("when".into(), "lunch".into())],
        },
        field: "protein".into(),
        from: None,
        to: None,
        group: None,
    }));
    assert_eq!(
        tsv_lines(&total),
        vec![vec!["field", "value"], vec!["protein", "49"]]
    );
}
