# Time

bottle stores UTC. It prints local time. Local time is the
timezone of the machine running bottle.

## Storage

`YYYY-MM-DDTHH:MM:SSZ`

Always that shape. Always seconds. Always `Z`. That string
sorts as an instant, so queries do not need a second epoch
column.

## Output

`YYYY-MM-DDTHH:MM:SS+ZZ:ZZ` in the host zone, including DST.
TSV never contains `Z`. Set the machine's timezone if you
want a particular city.

UTC strings sort. Local strings with mixed offsets do not
(a DST change will interleave `+10:00` and `+11:00`).
Conversion happens on the way out, and on the way in when
the caller did not send `Z`.

## Input

| Input | Meaning |
|---|---|
| omitted on `log` | now |
| `2026-08-21T22:14:00Z` | instant, stored as-is |
| `2026-08-22T08:14:00+10:00` | instant, converted to Z |
| `2026-08-22T08:14:00` | naive, host zone, then Z |
| `2026-08-22` | a civil day in the host zone |

A date-only value is a query bound, not an instant. `log`
and `amend --at` reject it.

Anything else is rejected: missing seconds, a space instead
of `T`, an offset without a colon.

## Ranges

`today`, `--from`, and `--to` are query windows, not columns.

A date becomes `[local midnight, next local midnight)` in
the host zone, then both ends are converted to UTC. Use the
zone database. Do not add a fixed offset. DST days are 23
or 25 hours.

`--from 2026-08-16 --to 2026-08-22` includes both civil
days. A full timestamptz on `--from` or `--to` is an
instant bound.

`--from` alone has no end. `--to` alone has no start.

`today` is `ls` for the current host civil day. It does not
print totals. Run `sum` for a total.
