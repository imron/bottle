# unignore

## Name

unignore — show an ignored entry in lists and totals again

## Synopsis

```
bottle unignore <schema> <id>
```

## Description

Clears `ignored` on an entry. The entry is again visible
to `ls`, `sum`, `last`, and `today`. Does not change
fields, `at`, `agent`, or links.

Running unignore on an id that is not ignored succeeds.

## Output

TSV of `id` and `at`.

## Exit status

`0` ok. `1` not found.

## See also

ignore, get, ls
