# today

## Name

today — list entries for the current civil day

## Synopsis

```
bottle today <schema> [--agent NAME] \
  [--where field=value]...
```

## Description

Lists entries of a schema whose `at` falls on the current
day in the timezone of this machine. It does not print a
total; run `sum` for that.

The window is local midnight through the next local
midnight. Days that change daylight saving time are 23 or
25 hours long.

Ignored entries are omitted.

## Options

`--agent NAME` — only entries written by that agent.

`--where field=value` — may repeat; all clauses are AND.
A declared field name filters that field. Any other name
is a link; the value must be `schema/id`.

## Output

Columns: `id`, `at`, `links`, the schema's fields in
declaration order, then `agent`.

## See also

ls, sum, last
