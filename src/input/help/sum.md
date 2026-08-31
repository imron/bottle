# sum

## Name

sum — total a number field

## Synopsis

```
bottle sum <schema> <field> [--from DATE|TIME] \
  [--to DATE|TIME] [--agent NAME] \
  [--where field=value]... \
  [--exclude field=value]... \
  [--link name=SCHEMA/ID]... \
  [--group day|week|month|year|<field>|<link>]
```

## Description

Totals a number field across entries of a schema. Only fields declared `number`
can be summed. `today` and `ls` do not print a total.

Fails if `<field>` is not a declared number. Ignored entries are omitted.

## Options

`--from DATE|TIME`, `--to DATE|TIME` — bound `at`. A date with no time of day is
that whole day in this machine's timezone, inclusive on both ends. A `YYYY-MM`
is that month. A full timestamp is an instant bound. Rows match on overlap.

`--agent NAME` — only entries written by that agent.

`--where field=value` — may repeat; all clauses are AND. The name must be a
declared field.

`--exclude field=value` — may repeat; a row drops if it matches any exclude.
Same field rules as `--where`.

`--link name=SCHEMA/ID` — may repeat; all clauses are AND. Entries that have
that named pointer.

`--group day|week|month|year` — bucket by the entry's time in the host timezone.
An event coarser than the group is omitted (`--group day` does not put a month
event on one day):

- `day` — `YYYY-MM-DD` (instant and day events)
- `week` — ISO `YYYY-Www` (instant and day events)
- `month` — `YYYY-MM`
- `year` — `YYYY`

`--group <field>|<link>` — a declared enum or text field, else a link name. Time
grains stay grains even if a field uses that name. Number fields are rejected.
The group column is that name. For a field the cell is the stored value; for a
link it is `schema/id`. An empty optional field is one group with an empty cell,
same as a missing link.

## Output

With no `--group`: columns `field` and `value`. An empty match still prints one
line with `value` `0`.

With `--group`: the group column and `value`. An empty match prints the header
and no data lines.

Numbers print as logged (`86.50` stays `86.50`).

## Examples

```
bottle sum nutrition.meal protein --from 2026-08-16 \
  --to 2026-08-22 --group day
bottle sum work.hours hours --link project=work.project/1
bottle sum work.hours hours --group project
bottle sum nutrition.fluid ml --from 2026-08 --group kind
```

## See also

ls, today
