# schema add

## Name

schema add — register a type from a YAML file

## Synopsis

```
bottle schema add <name> --file spec.yaml
```

## Description

Registers a new type. After this succeeds you can `log`
entries of that type.

`<name>` is one or more lowercase segments separated by
dots, for example `meal`, `nutrition.meal`, or
`fitness.strength.set`. Each segment is
`^[a-z][a-z0-9]*$`. `_` is `.` (`nutrition_meal` is
`nutrition.meal`). Empty segments are not allowed
(`meal.`, `.meal`, `foo..bar`). It must not already
exist.

The YAML is a field list:

```yaml
fields:
  - name: amount
    type: number
    required: true
  - name: kind
    type: enum
    required: true
    values: [in, out]
  - name: note
    type: text
    required: false
```

Types:

- `text` — a string. Comparison is case-sensitive. Tabs
  and newlines are rejected.
- `number` — a decimal, not scientific notation. Only
  number fields can be summed.
- `enum` — one of the listed values. Values are stored
  lowercase (`Water` becomes `water`). Duplicates after
  that fold are rejected.

Field names: `^[a-z][a-z0-9_]*$`. Reserved (you may not
use them as fields): `id`, `at`, `agent`, `ignored`,
`links`. There is no date field type; the time of the
event is `at` on every entry.

Links are not declared in the YAML. You set them on `log`
with `--link name=schema/id`.

On success the command prints nothing.

## Options

`--file spec.yaml` — path to the YAML field list
(required).

## Exit status

`0` ok. `1` bad name or schema already exists.

## See also

schema, schema show, log
