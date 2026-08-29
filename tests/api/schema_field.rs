use bottle::{Cmd, FieldType, cmd};

use crate::common::{MEAL, SESSION, SET, assert_fail, assert_usage, harness, tsv_lines};

#[test]
fn add_field_and_value() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
        schema: "nutrition.meal".into(),
        name: "fiber".into(),
        type_: FieldType::Number,
        values: None,
        default: None,
    })));
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddValue(cmd::SchemaAddValue {
        schema: "nutrition.meal".into(),
        field: "when".into(),
        value: "Brunch".into(),
    })));
    let show = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    assert!(show.contains("fiber\tnumber\tfalse\t"));
    assert!(show.contains("brunch"));
}

#[test]
fn add_value_duplicate_after_fold() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddValue(cmd::SchemaAddValue {
            schema: "nutrition.meal".into(),
            field: "when".into(),
            value: "BREAKFAST".into(),
        })))
        .unwrap_err();
    assert_fail(err, "exists");
}

#[test]
fn add_field_enum_requires_values() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "mood".into(),
            type_: FieldType::Enum,
            values: None,
            default: None,
        })))
        .unwrap_err();
    assert_usage(err, "values is required");
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "mood".into(),
            type_: FieldType::Enum,
            values: Some(vec![]),
            default: None,
        })))
        .unwrap_err();
    assert_usage(err, "values is required");
}

#[test]
fn add_field_values_only_for_enum() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "fiber".into(),
            type_: FieldType::Number,
            values: Some(vec!["1".into()]),
            default: None,
        })))
        .unwrap_err();
    assert_usage(err, "values is only valid");
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "note".into(),
            type_: FieldType::Text,
            values: Some(vec!["x".into()]),
            default: None,
        })))
        .unwrap_err();
    assert_usage(err, "values is only valid");
}

#[test]
fn add_field_duplicate_and_retired() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "kcal".into(),
            type_: FieldType::Number,
            values: None,
            default: None,
        })))
        .unwrap_err();
    assert_fail(err, "field exists");
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::Retire(cmd::SchemaRetire {
        name: "nutrition.meal".into(),
    })));
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "fiber".into(),
            type_: FieldType::Number,
            values: None,
            default: None,
        })))
        .unwrap_err();
    assert_fail(err, "retired");
}

#[test]
fn add_field_empty_default_is_usage() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "note".into(),
            type_: FieldType::Text,
            values: None,
            default: Some("".into()),
        })))
        .unwrap_err();
    assert_usage(err, "empty note");
}

#[test]
fn add_field_with_default() {
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
        Some("2026-08-22T08:00:00Z"),
    );
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
        schema: "nutrition.meal".into(),
        name: "note".into(),
        type_: FieldType::Text,
        values: None,
        default: Some("O'Brien".into()),
    })));
    let show = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    assert!(show.contains("note\ttext\ttrue\t"));
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
    let idx = lines[0].iter().position(|c| *c == "note").unwrap();
    assert_eq!(lines[1][idx], "O'Brien");
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
        schema: "nutrition.meal".into(),
        name: "fiber".into(),
        type_: FieldType::Number,
        values: None,
        default: Some("3".into()),
    })));
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
    let idx = lines[0].iter().position(|c| *c == "fiber").unwrap();
    assert_eq!(lines[1][idx], "3");
}

#[test]
fn add_field_enum_default_folds() {
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
        Some("2026-08-22T08:00:00Z"),
    );
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
        schema: "nutrition.meal".into(),
        name: "mood".into(),
        type_: FieldType::Enum,
        values: Some(vec!["happy".into(), "sad".into()]),
        default: Some("Happy".into()),
    })));
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
    let idx = lines[0].iter().position(|c| *c == "mood").unwrap();
    assert_eq!(lines[1][idx], "happy");
    assert!(!ls.contains("Happy"));
    let matched = h.run_ok(Cmd::Ls(cmd::Ls {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![("mood".into(), "happy".into())],
            links: vec![],
            excludes: vec![],
        },
        from: None,
        to: None,
        include_ignored: false,
    }));
    assert_eq!(tsv_lines(&matched).len(), 2);
}

#[test]
fn add_field_reserved_name() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    for name in ["id", "at", "agent", "ignored", "links", "grain"] {
        let err = h
            .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
                schema: "nutrition.meal".into(),
                name: name.into(),
                type_: FieldType::Text,
                values: None,
                default: None,
            })))
            .unwrap_err();
        assert_fail(err, "reserved field name");
    }
}

#[test]
fn add_value_not_enum_or_unknown() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddValue(cmd::SchemaAddValue {
            schema: "nutrition.meal".into(),
            field: "kcal".into(),
            value: "1".into(),
        })))
        .unwrap_err();
    assert_fail(err, "not enum");
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddValue(cmd::SchemaAddValue {
            schema: "nutrition.meal".into(),
            field: "nope".into(),
            value: "x".into(),
        })))
        .unwrap_err();
    assert_fail(err, "unknown field");
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::Retire(cmd::SchemaRetire {
        name: "nutrition.meal".into(),
    })));
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddValue(cmd::SchemaAddValue {
            schema: "nutrition.meal".into(),
            field: "when".into(),
            value: "brunch".into(),
        })))
        .unwrap_err();
    assert_fail(err, "retired");
}

#[test]
fn add_field_enum_and_bad_values() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
        schema: "nutrition.meal".into(),
        name: "mood".into(),
        type_: FieldType::Enum,
        values: Some(vec!["Happy".into(), "sad".into()]),
        default: None,
    })));
    let show = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    assert!(show.contains("mood\tenum\tfalse\thappy,sad"));
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "size".into(),
            type_: FieldType::Enum,
            values: Some(vec!["A".into(), "a".into()]),
            default: None,
        })))
        .unwrap_err();
    assert_fail(err, "duplicate enum value");
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "size".into(),
            type_: FieldType::Enum,
            values: Some(vec!["".into()]),
            default: None,
        })))
        .unwrap_err();
    assert_fail(err, "empty enum value");
}

#[test]
fn add_field_enum_trims_values() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
        schema: "nutrition.meal".into(),
        name: "mood".into(),
        type_: FieldType::Enum,
        values: Some(vec![" happy".into(), "sad ".into()]),
        default: None,
    })));
    let show = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    assert!(show.contains("mood\tenum\tfalse\thappy,sad"));
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "eggs"),
            ("kcal", "1"),
            ("protein", "1"),
            ("carbs", "0"),
            ("mood", "happy"),
        ],
        &[],
        Some("2026-08-22T08:00:00Z"),
    );
}

#[test]
fn add_field_rejects_existing_link_name() {
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
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "fitness.set".into(),
            name: "session".into(),
            type_: FieldType::Text,
            values: None,
            default: None,
        })))
        .unwrap_err();
    assert_fail(err, "collides with field");
}

#[test]
fn add_field_invalid_name() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "When".into(),
            type_: FieldType::Text,
            values: None,
            default: None,
        })))
        .unwrap_err();
    assert_fail(err, "invalid field name");
}

#[test]
fn add_value_empty() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddValue(cmd::SchemaAddValue {
            schema: "nutrition.meal".into(),
            field: "when".into(),
            value: "".into(),
        })))
        .unwrap_err();
    assert_fail(err, "empty enum value");
}

#[test]
fn add_value_rejects_comma() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddValue(cmd::SchemaAddValue {
            schema: "nutrition.meal".into(),
            field: "when".into(),
            value: "a,b".into(),
        })))
        .unwrap_err();
    assert_fail(err, "tab, newline, or comma");
}

fn rename(schema: &str, from: &str, to: &str) -> Cmd {
    Cmd::Schema(cmd::SchemaCmd::RenameField(cmd::SchemaRenameField {
        schema: schema.into(),
        from: from.into(),
        to: to.into(),
    }))
}

#[test]
fn rename_field_keeps_values_and_position() {
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
        ],
        &[],
        Some("2026-08-22T08:14:00Z"),
    );
    h.run_ok(rename("nutrition.meal", "kcal", "calories"));
    let show = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    let lines = tsv_lines(&show);
    assert_eq!(lines[3], vec!["calories", "number", "true", ""]);
    assert!(!show.contains("kcal"));
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
    let ls_lines = tsv_lines(&ls);
    assert!(ls_lines[0].contains(&"calories"), "{ls}");
    assert!(!ls.contains("kcal"));
    assert_eq!(
        ls_lines[1][ls_lines[0].iter().position(|c| *c == "calories").unwrap()],
        "568"
    );
    let sum = h.run_ok(Cmd::Sum(cmd::Sum {
        filters: cmd::Filters {
            schema: "nutrition.meal".into(),
            agent: None,
            wheres: vec![],
            links: vec![],
            excludes: vec![],
        },
        field: "calories".into(),
        from: None,
        to: None,
        group: None,
    }));
    assert_eq!(
        tsv_lines(&sum),
        vec![vec!["field", "value"], vec!["calories", "568"]]
    );
}

#[test]
fn rename_enum_keeps_values() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.run_ok(rename("nutrition.meal", "when", "slot"));
    let show = h.run_ok(Cmd::Schema(cmd::SchemaCmd::Show(cmd::SchemaShow {
        name: "nutrition.meal".into(),
        yaml: false,
    })));
    assert!(show.contains("slot\tenum\ttrue\tbreakfast,snack,lunch,dinner,extra"));
    h.log(
        "nutrition.meal",
        &[
            ("slot", "Lunch"),
            ("what", "rice"),
            ("kcal", "200"),
            ("protein", "10"),
            ("carbs", "40"),
        ],
        &[],
        Some("2026-08-22T12:00:00Z"),
    );
}

#[test]
fn rename_same_name_is_usage() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h.run(rename("nutrition.meal", "kcal", "kcal")).unwrap_err();
    assert_usage(err, "same field");
}

#[test]
fn rename_unknown_and_exists() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    let err = h
        .run(rename("nutrition.meal", "nope", "calories"))
        .unwrap_err();
    assert_fail(err, "unknown field");
    let err = h.run(rename("nutrition.meal", "kcal", "what")).unwrap_err();
    assert_fail(err, "field exists");
}

#[test]
fn rename_retired() {
    let mut h = harness();
    h.add_schema("nutrition.meal", MEAL);
    h.run_ok(Cmd::Schema(cmd::SchemaCmd::Retire(cmd::SchemaRetire {
        name: "nutrition.meal".into(),
    })));
    let err = h
        .run(rename("nutrition.meal", "kcal", "calories"))
        .unwrap_err();
    assert_fail(err, "retired");
}

#[test]
fn rename_collides_with_link() {
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
        .run(rename("fitness.set", "movement", "session"))
        .unwrap_err();
    assert_fail(err, "collides with field");
}
