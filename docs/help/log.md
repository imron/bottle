## log

### What

Writes one entry of a registered schema.

### Why

This is the write path. One event, declared fields, an
optional instant, optional named links. A correction later
is `amend`, not a second sentence in a note. Many entries
in one transaction: MCP `entries`, not a shell loop that
can stop halfway.

### How

```
bottle log <schema> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [field=value ...]
```

Required fields must be present. Unknown fields are
rejected. `text` may not contain tab or newline, and
compare is case-sensitive. `enum` values are folded
lowercase and must match a declared value (`Water` →
`water`). Fails if the schema is retired.

`--at` defaults to now. A date-only `--at` is an error
(that is a query bound, not an instant). Accepted
instants: `...Z`, `...+10:00`, or naive `YYYY-MM-DDTHH:MM:SS`
in the host zone. Always stored as UTC seconds `Z`.
Printed local, with offset, never `Z`.

`--agent` defaults to `BOTTLE_AGENT`. Empty if unset.

`--link session=fitness.session/1` points this entry at
another existing entry. Repeat `--link` for different
names. A name once per command. One name, one target, per
entry. The target must exist (ignored entries still
exist). A link
name uses the field-name regex, must not be a field on
this schema, and must not be reserved (`id`, `at`,
`agent`, `ignored`, `links`, `day`, `week`, `month`,
`year`).

Prints `id`, `at`, `links`:

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
