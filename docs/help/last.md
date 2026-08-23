## last

### What

Prints the newest entry of a schema, optionally filtered.

### Why

"When did I last speak to ada" is one entry, not a list.
Newest `at`, then highest `id` if two share an instant.

### How

```
bottle last <schema> [--agent NAME] \
  [--where field=value]...
```

Same columns as `ls`. Same `--agent` / `--where` rules.
Ignored entries are omitted. Exit 1 if none.

```
bottle last crm.touch --where who=ada
```
