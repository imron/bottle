## amend

### What

Changes an entry in place: fields, `at`, `agent`, links.

### Why

A correction is an edit, not another paragraph. The id
stays. `ignore` is for an entry that should vanish from
lists; `amend` is for an entry that should stay, fixed.
Does not clear `ignored`.

### How

```
bottle amend <schema> <id> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [--unlink name]... \
  [field=value ...]
```

At least one of `--at`, `--agent`, `--link`, `--unlink`,
or a `field=` is required. `--link` sets or replaces that
name's target. The target must exist. `--unlink name`
removes that name. Idempotent if the name is already
absent (still prints the entry). `--link` and `--unlink` of
the same name in one command is an error. Date-only
`--at` is an error. Prints `id`, `at`, `links`. Exit 1 if
missing.

```
bottle amend nutrition.fluid 1 ml=375
bottle amend fitness.set 2 --link session=fitness.session/9
bottle amend fitness.set 2 --unlink session
```
