use bottle::{Cmd, FieldType, cmd};

use crate::common::{MEAL, SESSION, SET, assert_fail, assert_usage, harness, tsv_lines};

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
        id: 1,
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
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::AddField(cmd::SchemaAddField {
            schema: "nutrition.meal".into(),
            name: "id".into(),
            type_: FieldType::Text,
            values: None,
            default: None,
        })))
        .unwrap_err();
    assert_fail(err, "reserved field name");
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
fn yaml_canonicalize_rejects() {
    let mut h = harness();
    for (name, yaml, needle) in [
        (
            "dup",
            "fields:\n  - name: title\n    type: text\n    required: false\n  - name: title\n    type: text\n    required: false\n",
            "duplicate field",
        ),
        (
            "enum_none",
            "fields:\n  - name: when\n    type: enum\n    required: true\n",
            "needs values",
        ),
        (
            "enum_dup",
            "fields:\n  - name: when\n    type: enum\n    required: true\n    values: [A, a]\n",
            "duplicate enum value",
        ),
        (
            "text_values",
            "fields:\n  - name: what\n    type: text\n    required: false\n    values: [x]\n",
            "only apply to enum",
        ),
        (
            "number_values",
            "fields:\n  - name: kcal\n    type: number\n    required: false\n    values: [x]\n",
            "only apply to enum",
        ),
        (
            "enum_empty_list",
            "fields:\n  - name: when\n    type: enum\n    required: true\n    values: []\n",
            "needs values",
        ),
        (
            "empty_enum",
            "fields:\n  - name: when\n    type: enum\n    required: true\n    values: ['']\n",
            "empty enum value",
        ),
        (
            "tab",
            "fields:\n  - name: when\n    type: enum\n    required: true\n    values: [\"a\\tb\"]\n",
            "tab, newline, or comma",
        ),
        (
            "comma",
            "fields:\n  - name: when\n    type: enum\n    required: true\n    values: ['a,b']\n",
            "tab, newline, or comma",
        ),
        (
            "unknown_field_key",
            "fields:\n  - name: kcal\n    type: number\n    required: true\n    default: 0\n",
            "unknown field",
        ),
        (
            "unknown_spec_key",
            "description: meals\nfields:\n  - name: kcal\n    type: number\n    required: true\n",
            "unknown field",
        ),
    ] {
        let file = h.yaml_file(&format!("{name}.yaml"), yaml);
        let err = h
            .run(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
                name: name.into(),
                file,
            })))
            .unwrap_err();
        assert_fail(err, needle);
    }
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
fn yaml_missing_and_invalid() {
    let mut h = harness();
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
            name: "nope".into(),
            file: h.dir.path().join("missing.yaml"),
        })))
        .unwrap_err();
    assert_fail(err, "file not found");
    let file = h.yaml_file("bad.yaml", "fields: [");
    let err = h
        .run(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
            name: "nope".into(),
            file,
        })))
        .unwrap_err();
    assert_fail(err, "invalid spec");
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
