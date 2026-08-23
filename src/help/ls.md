## ls

Lists entries of one schema, oldest first. This is the
read path for "what was logged." You do not write SQL.
Filters are a closed set: a time window, who wrote it,
field equality, and link equality.

```
bottle ls <schema> [--from DATE|TIME] [--to DATE|TIME] \
  [--agent NAME] [--where field=value]... \
  [--include-ignored]
```

Columns, left to right: `id`, `at`, `links`, then every
schema field in spec order, then `agent`. The schema name
is not repeated on each line. `ignored` is included only
with `--include-ignored`.

Ignored entries are omitted unless you pass that flag.
`get` can still fetch an ignored id.

Order is oldest `at`, then lowest `id`.

`--from` and `--to` bound `at`. A date with no time of
day is that civil day in the host timezone, inclusive on
both ends: `--from 2026-08-16 --to 2026-08-22` includes
both days. A full timestamp is an instant bound. `--from`
alone has no end. `--to` alone has no start. DST days are
23 or 25 hours; bottle uses the zone database, not a
fixed offset.

`--agent` filters who wrote the entry (the bookkeeping
column). On `log` and `amend` the same flag sets it.

`--where` may repeat; all clauses are AND. If the name is
a declared field, it filters that field (`enum` values
folded lowercase, `text` exact and case-sensitive).
Otherwise it is a link name and the value must be
`schema/id`. `--where` on `id`, `at`, `agent`, `ignored`,
or `links` is an error; use `--agent`, `get`, or
`--from` / `--to`.

```
bottle ls fitness.set --where session=fitness.session/1
bottle ls money.txn --from 2026-08-01 --to 2026-08-31 \
  --where kind=out
```

The `links` cell is space-separated `name=schema/id`
pairs, sorted by name. Empty means no links.
