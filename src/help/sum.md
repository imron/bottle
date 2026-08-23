# sum

## Name

sum — total a number field

## Synopsis

```
bottle sum <schema> <field> [--from DATE|TIME] \
  [--to DATE|TIME] [--agent NAME] \
  [--where field=value]... \
  [--group day|week|month|year|<link>]
```

## Description

Totals a number field across entries of a schema. This is
why the store is not a markdown diary: only fields
declared `number` can be summed. `today` and `ls` do not
print a total; run `sum` when you want one.

Fails if `<field>` is not a declared number. Ignored
entries are omitted.

## Options

`--from`, `--to`, `--agent`, `--where` — same as `ls`.

`--group day|week|month|year` — bucket by the entry's time
in the host timezone:

- `day` — `YYYY-MM-DD`
- `week` — ISO `YYYY-Www`
- `month` — `YYYY-MM`
- `year` — `YYYY`

`--group <link>` — any other name is a link name. The
group column is that name; the cell is `schema/id`.
Entries with no such link are one group with an empty
cell.

## Output

With no `--group`: columns `field` and `value`. An empty
match still prints one line with `value` `0`.

With `--group`: the group column and `value`. An empty
match prints the header and no data lines.

Numbers print without trailing zeros (`49` not `49.0`).

## Examples

```
bottle sum nutrition.meal protein --from 2026-08-16 \
  --to 2026-08-22 --group day
bottle sum work.hours hours --where project=work.project/1
bottle sum work.hours hours --group project
```

## See also

ls, today
