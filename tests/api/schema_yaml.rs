use bottle::{Cmd, cmd};

use crate::common::{assert_fail, harness};

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
