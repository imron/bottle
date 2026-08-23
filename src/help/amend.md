## amend

Changes an existing entry in place: fields, time, agent,
and links. The id stays. A correction is an edit, not
another paragraph.

`ignore` hides an entry from lists. `amend` keeps it
visible and fixes it. Amend does not clear `ignored`.

```
bottle amend <schema> <id> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [--unlink name]... \
  [field=value ...]
```

At least one of `--at`, `--agent`, `--link`, `--unlink`,
or a `field=value` is required.

`--link` sets or replaces that name's target. The target
entry must exist. `--unlink name` removes that name. If
the name is already absent, unlink still succeeds and
prints the entry. `--link` and `--unlink` of the same
name in one command is an error.

`--at` follows the same instant rules as `log`. A date
with no time of day is rejected.

Prints `id`, `at`, `links`. Exit 1 if the schema/id does
not exist.

```
bottle amend nutrition.fluid 1 ml=375
bottle amend fitness.set 2 --link session=fitness.session/9
bottle amend fitness.set 2 --unlink session
```
