# log

## Name

log — write one entry of a registered schema

## Synopsis

```
bottle log <schema> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [field=value ...]
```

## Description

Writes one entry of a registered schema. This is how a
fact gets into the store: one event, the fields you
declared, an optional time, an optional agent, and
optional named links to other entries.

A later correction is `amend` on this id, not a second
sentence in a note.

The schema must already exist and must not be retired.
Every required field must be present. Unknown field names
are rejected.

Field rules:

- `text` is case-sensitive. It may not contain a tab or a
  newline.
- `number` is an integer or float, not `1e3`, not
  infinity.
- `enum` values are folded to lowercase and must match a
  declared value (`Breakfast` is stored as `breakfast`).

To write many entries in one transaction (all succeed or
none do), use MCP `log` with `entries`. See `mcp`.

## Options

`--at TIME` — time of the event. Omit it and bottle uses
now. A date with no time of day is rejected here (that is
a query bound; see `ls`). Accepted instants:

- `2026-08-21T22:14:00Z` — UTC
- `2026-08-22T08:14:00+10:00` — offset, converted to UTC
- `2026-08-22T08:14:00` — no zone; treated as local time
  on this machine, then stored as UTC

Always stored as UTC seconds ending in `Z`. Always printed
in the host timezone with an offset, never `Z`. Seconds
are required. Use `T`, not a space. An offset must include
a colon (`+10:00`).

`--agent NAME` — who wrote the entry. Defaults to
`BOTTLE_AGENT`. If that is unset, the agent cell is empty.

`--link name=SCHEMA/ID` — point this entry at another
existing entry. The name is yours to choose (`session`,
`project`, `parent`). It is not declared in the schema
YAML. Repeat `--link` for different names. A name once per
command. One name, one target, per entry. The target must
exist (ignored entries still count). A link name uses the
field-name regex, must not collide with a field on this
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
