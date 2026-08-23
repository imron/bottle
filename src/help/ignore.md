# ignore

## Name

ignore — hide an entry from lists and totals

## Synopsis

```
bottle ignore <schema> <id>
```

## Description

Hides an entry from `ls`, `sum`, `last`, and `today`.
`get` still returns it. There is no un-ignore.

A bad fact should not be deleted: `schema drop` is for
types, and ignore is one-way. The entry is kept. If you
need the fact visible again, log a new entry.

Ignore does not clear links. Other entries that point here
still do, and those inbound links still block
`schema drop`.

Running ignore on an already-ignored id succeeds.

## Output

TSV of `id` and `at`.

## Exit status

`0` ok. `1` not found.

## See also

get, ls, schema drop, amend
