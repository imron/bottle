## ls

### What

Lists entries of a schema, oldest first.

### Why

The read path for "what was logged." Not SQL. Filters are
a closed set: time window, agent, field equality, link
equality. Ignored entries are omitted unless you ask for
them.

### How

```
bottle ls <schema> [--from DATE|TIME] [--to DATE|TIME] \
  [--agent NAME] [--where field=value]... \
  [--include-ignored]
```

Columns: `id`, `at`, `links`, schema fields in spec
order, `agent`. `ignored` only with `--include-ignored`.
Order: oldest `at`, then `id`. The schema name is not
repeated on each line.

`--from` / `--to` as a date are that civil day in the host
zone, inclusive on both ends. As a full timestamp, an
instant bound. `--from` alone has no end. `--to` alone
has no start.

`--agent` filters the bookkeeping column (who wrote the
entry). On `log` / `amend` the same flag sets it.

`--where` may repeat (AND). If the name is a declared
field, it filters that field (`enum` folded lowercase,
`text` exact). Otherwise it is a link name and the value
must be `schema/id`. `--where` on `id`, `at`, `agent`,
`ignored`, or `links` is an error.

```
bottle ls fitness.set --where session=fitness.session/1
bottle ls money.txn --from 2026-08-01 --to 2026-08-31 \
  --where kind=out
```

The `links` cell is space-separated `name=schema/id`
pairs, sorted by name. Empty means no links.
