use bottle::{Cmd, cmd};

use crate::common::{MEAL, SESSION, SET, assert_fail, eid, harness, tsv_lines};

#[test]
fn list_empty() {
    let mut h = harness();
    let out = h.run_ok(Cmd::Schema(cmd::SchemaCmd::List));
    assert_eq!(tsv_lines(&out), vec![vec!["name", "retired"]]);
}

#[test]
fn add_show_list() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let list = h.run_ok(Cmd::Schema(cmd::SchemaCmd::List));
    assert_eq!(
        tsv_lines(&list),
        vec![vec!["name", "retired"], vec!["nutrition.meal", "false"]]
    );
    let show = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    let lines = tsv_lines(&show);
    assert_eq!(lines[0], vec!["name", "type", "required", "values"]);
    assert_eq!(
        lines[1],
        vec!["when", "enum", "true", "breakfast,snack,lunch,dinner,extra"]
    );
    assert_eq!(lines[6], vec!["fat", "number", "false", ""]);
}

#[test]
fn show_yaml_round_trip() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let yaml = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: true,
    })));
    assert!(yaml.contains("name: when"));
    assert!(yaml.contains("type: enum"));
}

#[test]
fn add_rejects_bad_name() {
    let mut h = harness();
    let file = h.yaml_file("x.yaml", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
            name: "Meal".into(),
            file,
        })))
        .unwrap_err();
    assert_fail(err, "invalid schema name");
}

#[test]
fn add_accepts_one_segment_and_many() {
    let mut h = harness();
    h.add_schema("meal", MEAL);
    h.add_schema("fitness.strength.set", MEAL);
    let list = h.run_ok(Cmd::Schema(cmd::SchemaCmd::List));
    assert!(list.contains("meal\tfalse"));
    assert!(list.contains("fitness.strength.set\tfalse"));
}

#[test]
fn add_catalog_names_are_usable() {
    let mut h = harness();
    h.add_schema("links", SESSION);
    h.add_schema("schemas", SESSION);
    h.log(
        "links",
        &[("title", "x")],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
    let list = h.run_ok(Cmd::Schema(cmd::SchemaCmd::List));
    assert!(list.contains("links\tfalse"));
    assert!(list.contains("schemas\tfalse"));
    let ls = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "links".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    let lines = tsv_lines(&ls);
    assert_eq!(lines[1][3], "x");
}

#[test]
fn add_underscore_is_the_dotted_name() {
    let mut h = harness();
    h.add_schema("foo.bar", MEAL);
    let file = h.yaml_file("again.yaml", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
            name: "foo_bar".into(),
            file,
        })))
        .unwrap_err();
    assert_fail(err, "exists");
    let list = h.run_ok(Cmd::Schema(cmd::SchemaCmd::List));
    assert!(list.contains("foo.bar\tfalse"));
    assert!(!list.contains("foo_bar"));
}

#[test]
fn add_rejects_empty_segments() {
    let mut h = harness();
    for name in ["meal.", ".meal", "foo..bar", "foo__bar"] {
        let file = h.yaml_file(&format!("{name}.yaml"), MEAL);
        let err = h
            .run(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
                name: name.into(),
                file,
            })))
            .unwrap_err();
        assert_fail(err, "invalid schema name");
    }
}

#[test]
fn add_rejects_duplicate() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let file = h.yaml_file("again.yaml", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
            name: "nutrition.meal".into(),
            file,
        })))
        .unwrap_err();
    assert_fail(err, "exists");
}

#[test]
fn retire_blocks_log_not_show() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::Retire(cmd::SchemaRetire {
        name: "nutrition.meal".into(),
    })));
    let list = h.run_ok(Cmd::Schema(cmd::SchemaCmd::List));
    assert!(list.contains("nutrition.meal\ttrue"));
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    let err = h
        .run(Cmd::Log(cmd::Log {
            schema: "nutrition.meal".into(),
            at: None,
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
    assert_fail(err, "retired");
}

#[test]
fn drop_blocked_by_inbound_link() {
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
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Drop(cmd::SchemaDrop {
            name: "fitness.session".into(),
        })))
        .unwrap_err();
    assert_fail(err, "inbound");
    h.run_ok(Cmd::Amend(cmd::Amend {
        schema: "fitness.set".into(),
        id: eid(1),
        at: None,
        agent: None,
        links: vec![],
        unlinks: vec!["session".into()],
        fields: vec![],
    }));
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::Drop(cmd::SchemaDrop {
        name: "fitness.session".into(),
    })));
    let list = h.run_ok(Cmd::Schema(cmd::SchemaCmd::List));
    assert!(!list.contains("fitness.session"));
}

#[test]
fn show_missing() {
    let mut h = harness();
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
            name: "no.such".into(),
            yaml: false,
        })))
        .unwrap_err();
    assert_fail(err, "unknown schema");
}

#[test]
fn drop_unknown() {
    let mut h = harness();
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Drop(cmd::SchemaDrop {
            name: "no.such".into(),
        })))
        .unwrap_err();
    assert_fail(err, "unknown schema");
}

#[test]
fn retire_unknown() {
    let mut h = harness();
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Retire(cmd::SchemaRetire {
            name: "no.such".into(),
        })))
        .unwrap_err();
    assert_fail(err, "unknown schema");
}
