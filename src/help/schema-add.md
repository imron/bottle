## schema add

Registers a new type from a YAML file and creates a table
for it. After this succeeds you can `log` entries of that
type.

```
bottle schema add <name> --file spec.yaml
```

`<name>` must be `family.kind`: two lowercase identifiers
separated by a dot, for example `nutrition.meal`. It must
not already exist.

The YAML is a field list, not the event log:

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
- `number` — an integer or float, not scientific notation.
  Only number fields can be summed.
- `enum` — one of the listed values. Values are stored
  lowercase (`Water` becomes `water`). Duplicates after
  that fold are rejected.

Field names: `^[a-z][a-z0-9_]*$`. Reserved (you may not
use them as fields): `id`, `at`, `agent`, `ignored`,
`links`. There is no date field type; the time of the
event is `at` on every entry.

Links are not declared in the YAML. You set them on
`log` with `--link name=schema/id`.

On success the command prints nothing. On a bad name or
an existing schema it exits 1.
