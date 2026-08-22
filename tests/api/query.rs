use bottle::Cmd;

use crate::common::{self, MEAL, SESSION, SET, assert_fail, assert_usage, harness, tsv_lines};

fn seed_meals(h: &mut common::Harness) {
    h.add_schema("nutrition.meal", MEAL);
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "eggs"),
            ("kcal", "568"),
            ("protein", "49"),
            ("carbs", "5"),
            ("fat", "39.6"),
        ],
        &[],
        Some("2026-08-22T08:14:00Z"),
    );
    h.log(
        "nutrition.meal",
        &[
            ("when", "lunch"),
            ("what", "rice"),
            ("kcal", "200"),
            ("protein", "10"),
            ("carbs", "40"),
        ],
        &[],
        Some("2026-08-23T12:00:00Z"),
    );
}

#[test]
fn ls_columns_and_number_format() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        include_ignored: false,
    });
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
fn ls_instant_to_is_inclusive() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: Some("2026-08-22T08:14:00Z".into()),
        agent: None,
        wheres: vec![],
        include_ignored: false,
    });
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][4], "eggs");
}

#[test]
fn ls_from_to_dates() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: Some("2026-08-22".into()),
        to: Some("2026-08-22".into()),
        agent: None,
        wheres: vec![],
        include_ignored: false,
    });
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][4], "eggs");
}

#[test]
fn ls_where_enum_folds() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![("when".into(), "LUNCH".into())],
        include_ignored: false,
    });
    let lines = tsv_lines(&out);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][3], "lunch");
}

#[test]
fn ls_where_text_is_exact() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![("what".into(), "Eggs".into())],
        include_ignored: false,
    });
    assert_eq!(tsv_lines(&out).len(), 1);
}

#[test]
fn ls_where_agent_reserved() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Ls {
            schema: "nutrition.meal".into(),
            from: None,
            to: None,
            agent: None,
            wheres: vec![("agent".into(), "test".into())],
            include_ignored: false,
        })
        .unwrap_err();
    assert_usage(err, "reserved");
}

#[test]
fn ls_filter_agent_flag() {
    let mut h = harness();
    seed_meals(&mut h);
    let none = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: Some("other".into()),
        wheres: vec![],
        include_ignored: false,
    });
    assert_eq!(tsv_lines(&none).len(), 1);
    let some = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: Some("test".into()),
        wheres: vec![],
        include_ignored: false,
    });
    assert_eq!(tsv_lines(&some).len(), 3);
}

#[test]
fn get_includes_ignored() {
    let mut h = harness();
    seed_meals(&mut h);
    h.run_ok(Cmd::Ignore {
        schema: "nutrition.meal".into(),
        id: 1,
    });
    let ls = h.run_ok(Cmd::Ls {
        schema: "nutrition.meal".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        include_ignored: false,
    });
    assert_eq!(tsv_lines(&ls).len(), 2);
    let get = h.run_ok(Cmd::Get {
        schema: "nutrition.meal".into(),
        id: 1,
    });
    let lines = tsv_lines(&get);
    assert_eq!(lines[0].last().copied(), Some("ignored"));
    assert_eq!(lines[1].last().copied(), Some("true"));
}

#[test]
fn get_missing() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Get {
            schema: "nutrition.meal".into(),
            id: 99,
        })
        .unwrap_err();
    assert_fail(err, "not found");
}

#[test]
fn sum_and_group_day() {
    let mut h = harness();
    seed_meals(&mut h);
    let total = h.run_ok(Cmd::Sum {
        schema: "nutrition.meal".into(),
        field: "protein".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        group: None,
    });
    assert_eq!(
        tsv_lines(&total),
        vec![vec!["field", "value"], vec!["protein", "59"]]
    );
    let empty = h.run_ok(Cmd::Sum {
        schema: "nutrition.meal".into(),
        field: "protein".into(),
        from: Some("2020-01-01".into()),
        to: Some("2020-01-02".into()),
        agent: None,
        wheres: vec![],
        group: None,
    });
    assert_eq!(
        tsv_lines(&empty),
        vec![vec!["field", "value"], vec!["protein", "0"]]
    );
    let grouped = h.run_ok(Cmd::Sum {
        schema: "nutrition.meal".into(),
        field: "protein".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        group: Some("day".into()),
    });
    assert!(grouped.starts_with("day\tvalue\n"));
}

#[test]
fn sum_rejects_text_field() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Sum {
            schema: "nutrition.meal".into(),
            field: "what".into(),
            from: None,
            to: None,
            agent: None,
            wheres: vec![],
            group: None,
        })
        .unwrap_err();
    assert_fail(err, "not a number");
}

#[test]
fn last_newest() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Last {
        schema: "nutrition.meal".into(),
        agent: None,
        wheres: vec![],
    });
    let lines = tsv_lines(&out);
    assert_eq!(lines[1][4], "rice");
}

#[test]
fn last_empty() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Last {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
        })
        .unwrap_err();
    assert_fail(err, "not found");
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
    let ls = h.run_ok(Cmd::Ls {
        schema: "fitness.set".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![("session".into(), "fitness.session/1".into())],
        include_ignored: false,
    });
    let lines = tsv_lines(&ls);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1][2], "session=fitness.session/1");
    let grouped = h.run_ok(Cmd::Sum {
        schema: "fitness.set".into(),
        field: "reps".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        group: Some("session".into()),
    });
    assert!(grouped.contains("fitness.session/1\t8"));
}

#[test]
fn link_target_must_exist() {
    let mut h = harness();
    h.add_schema("fitness.set", SET);
    let err = h
        .run(Cmd::Log {
            schema: "fitness.set".into(),
            at: Some("2026-08-22T08:00:00Z".into()),
            agent: None,
            links: vec![("session".into(), "fitness.session/1".into())],
            fields: vec![
                ("movement".into(), "squat".into()),
                ("reps".into(), "8".into()),
            ],
        })
        .unwrap_err();
    assert_fail(err, "unknown schema");
}

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
    let out = h.run_ok(Cmd::Amend {
        schema: "fitness.set".into(),
        id: 1,
        at: None,
        agent: None,
        links: vec![],
        unlinks: vec!["session".into()],
        fields: vec![("reps".into(), "6".into())],
    });
    let lines = tsv_lines(&out);
    assert_eq!(lines[1][2], "");
    h.run_ok(Cmd::Amend {
        schema: "fitness.set".into(),
        id: 1,
        at: None,
        agent: None,
        links: vec![],
        unlinks: vec!["session".into()],
        fields: vec![],
    });
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
        .run(Cmd::Amend {
            schema: "fitness.set".into(),
            id: 1,
            at: None,
            agent: None,
            links: vec![("session".into(), "fitness.session/1".into())],
            unlinks: vec!["session".into()],
            fields: vec![],
        })
        .unwrap_err();
    assert_usage(err, "--link and --unlink");
}

#[test]
fn ignore_hides_from_sum() {
    let mut h = harness();
    seed_meals(&mut h);
    h.run_ok(Cmd::Ignore {
        schema: "nutrition.meal".into(),
        id: 1,
    });
    let total = h.run_ok(Cmd::Sum {
        schema: "nutrition.meal".into(),
        field: "protein".into(),
        from: None,
        to: None,
        agent: None,
        wheres: vec![],
        group: None,
    });
    assert_eq!(
        tsv_lines(&total),
        vec![vec!["field", "value"], vec!["protein", "10"]]
    );
}

#[test]
fn today_is_civil_day() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "now"),
            ("kcal", "1"),
            ("protein", "1"),
            ("carbs", "0"),
        ],
        &[],
        None,
    );
    let out = h.run_ok(Cmd::Today {
        schema: "nutrition.meal".into(),
        agent: None,
        wheres: vec![],
    });
    assert!(out.contains("now"));
}
