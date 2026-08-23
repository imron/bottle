## schema retire

### What

Blocks new `log`s on a schema. Existing entries stay
readable.

### Why

A type you no longer write should not disappear. `ls`,
`get`, and `sum` still work. Use this when you have
replaced the schema and want the old name kept as
history. Idempotent.

### How

```
bottle schema retire <name>
```

`schema add-field` and `schema add-value` also fail on a
retired schema. `amend`, `ignore`, and reads do not.
