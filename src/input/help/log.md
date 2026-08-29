# log

## Name

log — write one entry of a registered schema

## Synopsis

```
bottle log <schema> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [field=value ...]
```

## Description

Writes one entry of a registered schema: the fields you
declared, an optional time, an optional agent, and
optional named links to other entries.

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

`--at TIME` — time of the event. Omit it and bottle uses
now. A date with no time of day is not accepted. Seconds
are required. Use `T`, not a space. An offset must include
a colon (`+10:00`). Accepted instants:

- `2026-08-21T22:14:00Z` — UTC
- `2026-08-22T08:14:00+10:00` — with offset
- `2026-08-22T08:14:00` — local time on this machine

Printed times use this machine's timezone, with an offset.

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
`ignored`, `links`, `day`, `week`, `month`, `year`).

`field=value` — values for declared fields.

## Output

TSV of `id`, `at`, `links`.

## Exit status

`0` ok. `1` retired schema, unknown field, missing
required field, or missing link target. `2` bad time or
duplicate link name.

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

## See also

ls, amend, ignore, mcp, schema add
