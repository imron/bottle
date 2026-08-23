## schema retire

Stops new `log`s on a schema. Existing entries stay. You
can still `ls`, `get`, `sum`, `amend`, and `ignore`.

```
bottle schema retire <name>
```

Use this when you have replaced a type and want the old
name kept as history. Running it again on an already
retired schema succeeds (it is idempotent).

`schema add-field` and `schema add-value` also fail on a
retired schema. Exit 1 if the name is unknown.
