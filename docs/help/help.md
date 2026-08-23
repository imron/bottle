## help

### What

Prints the long explanation of a verb: what it does, why
it exists, how to use it.

### Why

`--help` is a usage line. A bot that has to guess from
flag names will invent SQL, skip `sum`, or log a paragraph
into `text`. This page is the contract in prose, inside
the binary, so the bot does not need the repo.

### How

```
bottle help
bottle help log
bottle help schema add
```

No TSV. `--db` is accepted and ignored: help does not
open the store.
