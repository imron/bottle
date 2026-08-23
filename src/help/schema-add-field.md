# schema add-field

## Name

schema add-field — add one field to an existing schema

## Synopsis

```
bottle schema add-field <name> --name <field> \
  --type text|number|enum [--values a,b] [--default N]
```

## Description

Adds one field to an existing schema. This is how a type
grows. You cannot rename a field, drop a field, or change
its type. For those, add a new schema, copy the entries
you still want, and `schema retire` the old name.

Without `--default` the new field is optional. Existing
entries have an empty cell there.

With `--default` the field is required and every existing
entry is filled with that value.

## Options

`--name <field>` — field name (`^[a-z][a-z0-9_]*$`).
Cannot be reserved: `id`, `at`, `agent`, `ignored`,
`links`.

`--type text|number|enum` — the field type.

`--values a,b` — required for `enum`, invalid for `text`
and `number`. Values are stored lowercase. Two values that
fold to the same lowercase string are rejected.

`--default N` — backfill existing entries and make the
field required.

## Exit status

`0` ok. `1` field exists, schema retired, or reserved
name.

## See also

schema add, schema add-value, schema retire
