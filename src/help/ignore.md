## ignore

### What

Hides an entry from `ls`, `sum`, `last`, and `today`.
`get` still returns it.

### Why

A bad entry should not be deleted: drop is for types, and
there is no un-ignore. The fact is kept. If you need it
visible again, log a new entry. `ignore` does not clear
links; inbound links still block `schema drop`.

### How

```
bottle ignore <schema> <id>
```

Sets `ignored`. Idempotent. Prints `id`, `at`. Exit 1 if
missing.
