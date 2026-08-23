## sum

### What

Totals a declared number field.

### Why

This is why the store is not markdown. Only fields
declared `number` can be summed. Bots should not multiply
in their head when the type already says the field is
additive. `today` does not print a total; run `sum`.

### How

```
bottle sum <schema> <field> [--from DATE|TIME] \
  [--to DATE|TIME] [--agent NAME] \
  [--where field=value]... \
  [--group day|week|month|year|<link>]
```

Fails if `<field>` is not a declared number. `--from`,
`--to`, `--agent`, `--where`: same as `ls`. Ignored
entries are omitted.

With no `--group`: columns `field`, `value`. An empty set
is one line, `value` `0`.

`--group day|week|month|year` uses the host zone:

- `day` -- `YYYY-MM-DD`
- `week` -- ISO `YYYY-Www`
- `month` -- `YYYY-MM`
- `year` -- `YYYY`

Any other `--group` name is a link name. The group column
is that name; the cell is `schema/id`. Entries with no
such link are one group with an empty cell. An empty set
with `--group` prints the header and no lines.

```
bottle sum nutrition.meal protein --from 2026-08-16 \
  --to 2026-08-22 --group day
bottle sum work.hours hours --where project=work.project/1
bottle sum work.hours hours --group project
```

Numbers print without trailing zeros (`49` not `49.0`).
