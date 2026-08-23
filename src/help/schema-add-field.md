## schema add-field

Adds one field to an existing schema. This is how a type
grows. You cannot rename a field, drop a field, or change
its type. For those, add a new schema, copy the entries
you still want, and `schema retire` the old name.

```
bottle schema add-field <name> --name <field> \
  --type text|number|enum [--values a,b] [--default N]
```

Without `--default` the new field is optional. Existing
entries have an empty cell there.

With `--default` the field is required and every existing
entry is filled with that value.

`--values` is required for `enum`, and invalid for `text`
and `number`. Enum values are stored lowercase. Two values
that fold to the same lowercase string are rejected.

Fails if the field already exists, the schema is retired,
or the name is reserved (`id`, `at`, `agent`, `ignored`,
`links`).
