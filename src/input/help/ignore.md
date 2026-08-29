# ignore

## Name

ignore — hide an entry from lists and totals

## Synopsis

```
bottle ignore <schema> <id>
```

## Description

Hides an entry from `ls`, `sum`, `last`, and `today`.
`get` still returns it. `unignore` clears `ignored` so
those queries see the entry again.

Ignore does not clear links. Other entries that point here
still do, and those links still block `schema drop`.

Running ignore on an already-ignored id succeeds.

## Output

TSV of `id` and `at`.

## Exit status

`0` ok. `1` not found.

## See also

get, ls, unignore, schema drop, amend
