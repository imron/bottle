## schema add

### What

Registers a type from a YAML file and creates its table.

### Why

A log with undeclared fields is a note. Adding the schema
is what makes later `sum` and `--where` safe. The YAML is
the field list, not the event log.

### How

```
bottle schema add <name> --file spec.yaml
```

`<name>` must be `family.kind` and must match
`^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*$`. Fails if the name
exists.

YAML:

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

Types: `text`, `number`, `enum`. Enum values are stored
lowercase (`Water` → `water`). Text compare is
case-sensitive. Field names:
`^[a-z][a-z0-9_]*$`. Reserved field names: `id`, `at`,
`agent`, `ignored`, `links`. No date field; time is `at`.
Links are not declared here; they are set on `log`.
