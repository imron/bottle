# Time

bottle stores UTC. It prints local time. Local time is the
timezone of the machine running bottle.

`--at` parses by shape. No `--grain` flag.

## Storage

UTC start plus the grain.

The start is `YYYY-MM-DDTHH:MM:SSZ`. Always that shape.
Always seconds. Always `Z`. That string sorts as an
instant. Grain is `instant`, `day`, or `month`. Year, ISO
week, and calendar quarter wait. A BAS quarter is a schema
field, not an `--at` grain.

## Output

Print in the same shape as the input grain:

| Grain | Printed `at` |
|---|---|
| instant | `YYYY-MM-DDTHH:MM:SS+ZZ:ZZ` in the host zone, including DST |
| day | `YYYY-MM-DD` |
| month | `YYYY-MM` |

TSV never contains `Z`. Set the machine's timezone if you
want a particular city.

UTC strings sort. Local strings with mixed offsets do not
(a DST change will interleave `+10:00` and `+11:00`).
Conversion happens on the way out, and on the way in when
the caller did not send `Z`.

## Input

| Input | Meaning |
|---|---|
| omitted on `log` | now, an instant |
| `2026-08-21T22:14:00Z` | instant, stored as-is |
| `2026-08-22T08:14:00+10:00` | instant, converted to Z |
| `2026-08-22T08:14:00` | naive, host zone, then Z |
| `2026-08-22` | a civil day in the host zone |
| `2026-08` | a calendar month in the host zone |

Anything else is rejected: missing seconds, a space instead
of `T`, an offset without a colon, a year, an ISO week, or
a quarter.

## Ranges

`today`, `--from`, and `--to` are query windows, not columns.

A date becomes `[local midnight, next local midnight)` in
the host zone, then both ends are converted to UTC. A
`YYYY-MM` is that calendar month. Use the zone database.
Do not add a fixed offset. DST days are 23 or 25 hours.

`--from` / `--to` match on overlap: a month event in
August matches `--from 2026-08-21 --to 2026-08-21`. A full
timestamptz on `--from` or `--to` is an instant bound.

`--from 2026-08-16 --to 2026-08-22` includes both civil
days.

`--from` alone has no end. `--to` alone has no start.

`today` is the civil day only: instants and day events on
the current host civil day. A month event does not appear.
It does not print totals. Run `sum` for a total.

`sum --group day` (and `week`) omit events coarser than
the group. A month event is not put on one day.
