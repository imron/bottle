# last

## Name

last — print the most recent entry of a schema

## Synopsis

```
bottle last <schema> [--agent NAME] \
  [--where field=value]...
```

## Description

Prints the most recent entry of a schema, optionally
filtered. One result, not a list.

Most recent `at` wins. If two entries share an instant,
the highest `id` wins. Ignored entries are omitted.

## Options

`--agent NAME` — only entries written by that agent.

`--where field=value` — may repeat; all clauses are AND.
A declared field name filters that field. Any other name
is a link; the value must be `schema/id`.

## Output

Columns: `id`, `at`, `links`, the schema's fields in
declaration order, then `agent`.

## Exit status

`0` ok. `1` nothing matches.

## Examples

```
bottle last crm.touch --where who=ada
```

## See also

ls, get, today
