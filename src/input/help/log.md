# log

## Name

log — write one entry of a registered schema

## Synopsis

```
bottle log <schema> [--at DATE|TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [field=value ...]
bottle log <schema> --file rows.tsv
bottle log <schema> --file -
```

## Description

Writes one entry of a registered schema: the fields you
declared, an optional time, an optional agent, and
optional named links to other entries.

`--file` logs many entries in one transaction from a TSV
file. `--file -` reads stdin. The schema is on the
command. The first row is a header. Columns may be `at`,
`agent`, `links`, and declared field names. The `links`
cell is the same space-separated `name=schema/id` form as
output.

`--at`, `--agent`, `--link`, and `field=value` on the
command are defaults for every row. A non-empty TSV cell
wins. Missing `at` (no cell and no `--at`) is now.
Missing `agent` is `BOTTLE_AGENT`. Header with no data
rows is an error.

Use `amend` to correct an entry if needed.

The schema must already exist and must not be retired.
Every required field must be present. Unknown field names
are rejected.

Field rules:

- `text` is case-sensitive. It may not contain a tab or a
  newline.
- `number` is an integer or float in the form bottle
  stores (`1`, `1.10`), not `1e3`, `01`, or `+1`.
- `enum` values are folded to lowercase and must match a
  declared value (`Breakfast` is stored as `breakfast`).
  Tab, newline, and comma are rejected.

## Options

`--at DATE|TIME` — when the event happened. Shape chooses
the grain. Omit it and bottle uses now (an instant). Date
and time may be split by `T` or a space (quote the value
on the CLI). Seconds are optional (missing seconds are
`:00`). Offset is `Z` (UTC), `+10:00`, `+1000`, or `+10`.
No offset is the host zone. Year, ISO week, and quarter
are not grains.

- `2026-08-21T22:14:00Z` — instant, UTC
- `2026-08-22T08:14:00+10:00` — instant, with offset
- `2026-08-22T08:14` — instant, seconds default to 00
- `2026-08-22 08:14:00` — instant, space instead of `T`
- `2026-08-22T08:14:00` — instant, local time on this machine
- `2026-08-22` — a civil day in the host zone
- `2026-08` — a calendar month in the host zone

Printed `at` uses the same shape: a day comes back
`2026-08-22`, not midnight. Instants print in this
machine's timezone, with an offset.

`--agent NAME` — who wrote the entry. Defaults to
`BOTTLE_AGENT`, or `bottle` if that is unset. Leading and
trailing spaces are stripped. May not contain a tab or a
newline.

`--link name=SCHEMA/ID` — point this entry at another
existing entry. The name is yours to choose (`session`,
`project`, `parent`). It is not declared in the schema
YAML. Repeat `--link` for different names. A name once per
command. One name, one target, per entry. The target must
exist (ignored entries still count). A link name starts
with a lowercase letter, then letters, digits, or
underscores. It must not collide with a field on this
schema, and must not be reserved (`id`, `at`, `agent`,
`ignored`, `links`, `grain`, `day`, `week`, `month`,
`year`).

`field=value` — values for declared fields.

`--file PATH` — TSV of entries. `-` is stdin. Header
required. All rows succeed or none do. Other log flags
are defaults for blank cells. Errors name the file line
(header is line 1). Trailing empty cells are not extra
columns. A row with fewer cells than the header is an
error; extra cells with values are an error.

## Output

TSV of `id`, `at`, `links`.

## Exit status

`0` ok. `1` retired schema, unknown field, missing
required field, or missing link target. `2` bad time,
duplicate link name, or a bad `--file`.

## Examples

```
bottle log crm.touch who=ada channel=email
```

```
id	at	links
1	2026-08-22T08:14:00+10:00	
```

```
bottle log fitness.set --link session=fitness.session/1 \
  movement=squat reps=8 load=24 unit=kg
```

```
id	at	links
1	2026-08-22T08:14:00+10:00	session=fitness.session/1
```

```
bottle log nutrition.meal --file meals.tsv
```

## See also

ls, amend, ignore, unignore, mcp, schema add
