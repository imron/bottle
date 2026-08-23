## sum

Totals a number field across entries of a schema. This is
why the store is not a markdown diary: only fields
declared `number` can be summed. `today` and `ls` do not
print a total; run `sum` when you want one.

```
bottle sum <schema> <field> [--from DATE|TIME] \
  [--to DATE|TIME] [--agent NAME] \
  [--where field=value]... \
  [--group day|week|month|year|<link>]
```

Fails if `<field>` is not a declared number. `--from`,
`--to`, `--agent`, and `--where` work as on `ls`. Ignored
entries are omitted.

With no `--group` the table has columns `field` and
`value`. An empty match still prints one line with
`value` `0`.

`--group day`, `week`, `month`, or `year` buckets by the
entry's time in the host timezone:

- `day` — `YYYY-MM-DD`
- `week` — ISO `YYYY-Www`
- `month` — `YYYY-MM`
- `year` — `YYYY`

Any other `--group` name is a link name. The group column
is that name; the cell is `schema/id`. Entries with no
such link are one group with an empty cell. An empty
match with `--group` prints the header and no data lines.

```
bottle sum nutrition.meal protein --from 2026-08-16 \
  --to 2026-08-22 --group day
bottle sum work.hours hours --where project=work.project/1
bottle sum work.hours hours --group project
```

Numbers print without trailing zeros (`49` not `49.0`).
