# amend

## Name

amend — change an existing entry in place

## Synopsis

```
bottle amend <schema> <id> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [--unlink name]... \
  [field=value ...]
```

## Description

Changes an existing entry: fields, time, agent, and links.
The id stays.

Amend does not clear `ignored`. Use `ignore` to hide an
entry and `unignore` to show it again.

At least one of `--at`, `--agent`, `--link`, `--unlink`,
or a `field=value` is required.

## Options

`--at TIME` — time of the event. A date with no time of
day is not accepted. Seconds are required. Use `T`, not a
space. An offset must include a colon (`+10:00`). UTC
(`Z`), an offset (`+10:00`), or local time with no zone
are accepted.

`--agent NAME` — set who wrote the entry.

`--link name=SCHEMA/ID` — set or replace the target for
the named link. The target entry must exist.

`--unlink name` — remove that named link. If the named
link is already absent, unlink still succeeds and prints
the entry. `--link` and `--unlink` of the same name in one
command is an error.

`field=value` — fields to change.

## Output

TSV of `id`, `at`, `links`.

## Exit status

`0` ok. `1` not found. `2` no change given, or `--link`
and `--unlink` of the same name.

## Examples

```
bottle amend nutrition.fluid 1 ml=375
bottle amend fitness.set 2 --link session=fitness.session/9
bottle amend fitness.set 2 --unlink session
```

## See also

log, ignore, unignore, get
