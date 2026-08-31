# today

## Name

today — list entries for the current civil day

## Synopsis

```
bottle today <schema> [--agent NAME] \
  [--where field=value]... [--exclude field=value]... \
  [--link name=SCHEMA/ID]...
```

## Description

Lists entries of a schema whose `at` is an instant or a civil day on the current
day in the timezone of this machine. Month events are omitted even if the month
contains today. It does not print a total; run `sum` for that.

The window is local midnight through the next local midnight. Days that change
daylight saving time are 23 or 25 hours long.

Ignored entries are omitted.

## Options

`--agent NAME` — only entries written by that agent.

`--where field=value` — may repeat; all clauses are AND. The name must be a
declared field.

`--exclude field=value` — may repeat; a row drops if it matches any exclude.
Same field rules as `--where`.

`--link name=SCHEMA/ID` — may repeat; all clauses are AND. Entries that have
that named pointer.

## Output

Columns: `id`, `at`, `links`, the schema's fields in declaration order, then
`agent`.

## See also

ls, sum, last
