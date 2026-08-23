# schema retire

## Name

schema retire — block new logs on a schema

## Synopsis

```
bottle schema retire <name>
```

## Description

Stops new `log`s on a schema. Existing entries stay. You
can still `ls`, `get`, `sum`, `amend`, and `ignore`.

Use this when you have replaced a type and want the old
name kept as history. Running it again on an already
retired schema succeeds.

`schema add-field` and `schema add-value` also fail on a
retired schema.

## Exit status

`0` ok. `1` unknown schema.

## See also

schema drop, schema add, log
