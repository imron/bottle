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
The id stays. A correction is an edit, not another
paragraph.

Amend does not clear `ignored`. Use the `ignore` command
to hide an entry.

At least one of `--at`, `--agent`, `--link`, `--unlink`,
or a `field=value` is required.

## Options

`--at TIME` — same instant rules as `log`. A date with no
time of day is rejected.

`--agent NAME` — set who wrote the entry.

`--link name=SCHEMA/ID` — set or replace that name's
target. The target entry must exist.

`--unlink name` — remove that name. If the name is already
absent, unlink still succeeds and prints the entry.
`--link` and `--unlink` of the same name in one command is
an error.

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

log, ignore, get
