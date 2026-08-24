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

Totals a number field across entries of a schema. Only
fields declared `number` can be summed. `today` and `ls`
do not print a total.

Fails if `<field>` is not a declared number. Ignored
entries are omitted.

## Options

`--from DATE|TIME`, `--to DATE|TIME` — bound `at`. A date
with no time of day is that whole day in this machine's
timezone, inclusive on both ends. A full timestamp is an
instant bound.

`--agent NAME` — only entries written by that agent.

`--where field=value` — may repeat; all clauses are AND.
A declared field name filters that field. Any other name
is a link; the value must be `schema/id`.

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

Numbers print as logged (`86.50` stays `86.50`).

## Examples

```
bottle sum nutrition.meal protein --from 2026-08-16 \
  --to 2026-08-22 --group day
bottle sum work.hours hours --where project=work.project/1
bottle sum work.hours hours --group project
```

## See also

ls, today
