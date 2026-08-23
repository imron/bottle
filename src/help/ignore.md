## ignore

Hides an entry from `ls`, `sum`, `last`, and `today`.
`get` still returns it. There is no un-ignore.

```
bottle ignore <schema> <id>
```

A bad fact should not be deleted: `schema drop` is for
types, and ignore is one-way. The entry is kept. If you
need the fact visible again, log a new entry.

Ignore does not clear links. Other entries that point here
still do, and those inbound links still block
`schema drop`.

Running ignore on an already-ignored id succeeds
(idempotent). Prints `id` and `at`. Exit 1 if the
schema/id does not exist.
