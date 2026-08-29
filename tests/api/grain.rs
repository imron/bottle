use bottle::{Cmd, cmd};

use crate::common::{MEAL, assert_usage, harness, tsv_lines};

fn meal(h: &mut crate::common::Harness, at: &str, what: &str, kcal: &str) {
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", what),
            ("kcal", kcal),
            ("protein", "1"),
            ("carbs", "0"),
        ],
        &[],
        Some(at),
    );
}

fn ls_whats(h: &mut crate::common::Harness, from: Option<&str>, to: Option<&str>) -> Vec<String> {
    let out = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: from.map(str::to_string),
        to: to.map(str::to_string),
        include_ignored: false,
    }));
    tsv_lines(&out)
        .into_iter()
        .skip(1)
        .map(|row| row[4].to_string())
        .collect()
}

#[test]
fn month_at_prints_yyyy_mm() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let out = h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "rent"),
            ("kcal", "1"),
            ("protein", "1"),
            ("carbs", "0"),
        ],
        &[],
        Some("2026-08"),
    );
    assert_eq!(tsv_lines(&out)[1][1], "2026-08");
}

#[test]
fn from_to_match_on_overlap() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    meal(&mut h, "2026-08-22T08:14:00Z", "instant", "1");
    meal(&mut h, "2026-08-22", "day", "10");
    meal(&mut h, "2026-08", "month", "100");
    let mut day = ls_whats(&mut h, Some("2026-08-22"), Some("2026-08-22"));
    day.sort();
    assert_eq!(day, vec!["day", "instant", "month"]);
    assert_eq!(
        ls_whats(&mut h, Some("2026-08-23"), Some("2026-08-23")),
        vec!["month"]
    );
    assert!(ls_whats(&mut h, Some("2026-09"), Some("2026-09")).is_empty());
    let mut august = ls_whats(&mut h, Some("2026-08"), Some("2026-08"));
    august.sort();
    assert_eq!(august, vec!["day", "instant", "month"]);
}

#[test]
fn today_is_civil_day_not_month() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let posted = h.log(
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
    let at = tsv_lines(&posted)[1][1];
    let day = &at[..10];
    let month = &at[..7];
    meal(&mut h, day, "day", "10");
    meal(&mut h, month, "month", "100");
    let out = h.run_ok(Cmd::Today(cmd::Filters {
        schema: "nutrition.meal".into(),
        agent: None,
        wheres: vec![],
        links: vec![],
    }));
    assert!(out.contains("now"), "{out}");
    assert!(out.contains("day"), "{out}");
    assert!(!out.contains("month"), "{out}");
}

#[test]
fn sum_group_day_skips_month() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    meal(&mut h, "2026-08-22T08:14:00Z", "instant", "1");
    meal(&mut h, "2026-08-22", "day", "10");
    meal(&mut h, "2026-08", "month", "100");
    let by_day = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        field: "kcal".into(),
        from: None,
        to: None,
        group: Some("day".into()),
    }));
    assert_eq!(
        tsv_lines(&by_day),
        vec![vec!["day", "value"], vec!["2026-08-22", "11"]]
    );
    let by_month = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        field: "kcal".into(),
        from: None,
        to: None,
        group: Some("month".into()),
    }));
    assert_eq!(
        tsv_lines(&by_month),
        vec![vec!["month", "value"], vec!["2026-08", "111"]]
    );
    let total = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        field: "kcal".into(),
        from: Some("2026-08-22".into()),
        to: Some("2026-08-22".into()),
        group: None,
    }));
    assert_eq!(
        tsv_lines(&total),
        vec![vec!["field", "value"], vec!["kcal", "111"]]
    );
}

#[test]
fn amend_at_can_be_a_day() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    meal(&mut h, "2026-08-22T08:14:00Z", "eggs", "1");
    let out = h.run_ok(Cmd::Amend(cmd::Amend {
        schema: "nutrition.meal".into(),
        id: crate::common::eid(1),
        at: Some("2026-08-22".into()),
        agent: None,
        links: vec![],
        unlinks: vec![],
        fields: vec![],
    }));
    assert_eq!(tsv_lines(&out)[1][1], "2026-08-22");
}

#[test]
fn year_week_and_quarter_are_not_grains() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    for at in ["2026", "2026-W34", "2026-Q3"] {
        let err = h
            .run(Cmd::Log(cmd::Log {
                schema: "nutrition.meal".into(),
                at: Some(at.into()),
                agent: None,
                links: vec![],
                file: None,
                fields: vec![
                    ("when".into(), "breakfast".into()),
                    ("what".into(), "x".into()),
                    ("kcal".into(), "1".into()),
                    ("protein".into(), "1".into()),
                    ("carbs".into(), "0".into()),
                ],
            }))
            .unwrap_err();
        assert_usage(err, "invalid time");
    }
}
