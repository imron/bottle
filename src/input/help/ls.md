# ls

## Name

ls — list entries of a schema

## Synopsis

```
bottle ls <schema> [--from DATE|TIME] [--to DATE|TIME] \
  [--agent NAME] [--where field=value]... \
  [--include-ignored]
```

## Description

Lists entries of one schema, oldest first. You can filter
by a time window, who wrote the entry, field equality, and
link equality.

Ignored entries are omitted unless `--include-ignored`.
`get` can still fetch an ignored id.

Order is oldest `at`, then lowest `id`. The schema name is
not repeated on each line.

## Options

`--from DATE|TIME` — lower bound on `at`. A date with no
time of day is that civil day in the host timezone,
inclusive. A full timestamp is an instant bound. `--from`
alone has no end.

`--to DATE|TIME` — upper bound on `at`. A date is that
civil day, inclusive. `--from 2026-08-16 --to 2026-08-22`
includes both days. `--to` alone has no start. Days that
change daylight saving time are 23 or 25 hours long.

`--agent NAME` — filter who wrote the entry. On `log` and
`amend` the same flag sets it.

`--where field=value` — may repeat; all clauses are AND.
If the name is a declared field, it filters that field
(`enum` values folded lowercase, `text` exact and
case-sensitive). Otherwise it is a link name and the value
must be `schema/id`. `--where` on `id`, `at`, `agent`,
`ignored`, or `links` is an error; use `--agent`, `get`,
or `--from` / `--to`.

`--include-ignored` — include ignored entries and print
the `ignored` column.

## Output

Columns, left to right: `id`, `at`, `links`, then every
schema field in spec order, then `agent`. `ignored` only
with `--include-ignored`.

The `links` cell is space-separated `name=schema/id`
pairs, sorted by name. Empty means no links.

## Examples

```
bottle ls fitness.set --where session=fitness.session/1
bottle ls money.txn --from 2026-08-01 --to 2026-08-31 \
  --where kind=out
```

## See also

get, last, today, sum, log
