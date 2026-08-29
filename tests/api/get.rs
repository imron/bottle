use bottle::{Cmd, cmd};

use crate::common::{MEAL, assert_fail, eid, harness, seed_meals, tsv_lines};

#[test]
fn get_includes_ignored() {
    let mut h = harness();
    seed_meals(&mut h);
    h.run_ok(Cmd::Ignore(cmd::Ignore {
        schema: "nutrition.meal".into(),
        id: eid(1),
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
    assert_eq!(tsv_lines(&ls).len(), 2);
    let get = h.run_ok(Cmd::Get(cmd::Get {
        schema: "nutrition.meal".into(),
        id: eid(1),
    }));
    let lines = tsv_lines(&get);
    assert_eq!(lines[0].last().copied(), Some("ignored"));
    assert_eq!(lines[1].last().copied(), Some("true"));
}

#[test]
fn get_rejects_zero_id() {
    use bottle::{EntryId, Error, Usage};
    assert!(matches!(
        EntryId::parse(0).unwrap_err(),
        Error::Usage(Usage::InvalidEntryId(0))
    ));
}

#[test]
fn get_missing() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Get(cmd::Get {
            schema: "nutrition.meal".into(),
            id: eid(99),
        }))
        .unwrap_err();
    assert_fail(err, "not found");
}

#[test]
fn last_newest() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Last(cmd::Filters {
        schema: "nutrition.meal".into(),
        agent: None,
        wheres: vec![],
        links: vec![],
        excludes: vec![],
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines[1][4], "rice");
}

#[test]
fn last_exclude_skips_matching_newest() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Last(cmd::Filters {
        schema: "nutrition.meal".into(),
        agent: None,
        wheres: vec![],
        links: vec![],
        excludes: vec![("when".into(), "lunch".into())],
    }));
    let lines = tsv_lines(&out);
    assert_eq!(lines[1][4], "eggs");
}

#[test]
fn last_empty() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Last(cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        }))
        .unwrap_err();
    assert_fail(err, "not found");
}

#[test]
fn ignore_hides_from_sum() {
    let mut h = harness();
    seed_meals(&mut h);
    h.run_ok(Cmd::Ignore(cmd::Ignore {
        schema: "nutrition.meal".into(),
        id: eid(1),
    }));
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
    let out = h.run_ok(Cmd::Today(cmd::Filters {
        schema: "nutrition.meal".into(),
        agent: None,
        wheres: vec![],
        links: vec![],
        excludes: vec![],
    }));
    assert!(out.contains("now"));
}

#[test]
fn ignore_missing() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Ignore(cmd::Ignore {
            schema: "nutrition.meal".into(),
            id: eid(99),
        }))
        .unwrap_err();
    assert_fail(err, "not found");
}

#[test]
fn unignore_shows_in_ls_and_sum() {
    let mut h = harness();
    seed_meals(&mut h);
    h.run_ok(Cmd::Ignore(cmd::Ignore {
        schema: "nutrition.meal".into(),
        id: eid(1),
    }));
    h.run_ok(Cmd::Unignore(cmd::Unignore {
        schema: "nutrition.meal".into(),
        id: eid(1),
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
    assert_eq!(tsv_lines(&ls).len(), 3);
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
}

#[test]
fn unignore_is_idempotent() {
    let mut h = harness();
    seed_meals(&mut h);
    let out = h.run_ok(Cmd::Unignore(cmd::Unignore {
        schema: "nutrition.meal".into(),
        id: eid(1),
    }));
    assert!(out.starts_with("id\tat\n"), "{out}");
}

#[test]
fn unignore_missing() {
    let mut h = harness();
    seed_meals(&mut h);
    let err = h
        .run(Cmd::Unignore(cmd::Unignore {
            schema: "nutrition.meal".into(),
            id: eid(99),
        }))
        .unwrap_err();
    assert_fail(err, "not found");
}
