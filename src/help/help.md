## help

Prints the long explanation of a command: what it is for,
the flags, the rules, and an example. This is the page a
person or a bot should read before using a verb.

`--help` on a command is only a short usage line (flag
names). `bottle help` is the full page, compiled into the
binary, so you do not need this repository open.

```
bottle help
bottle help log
bottle help schema add
```

With no topic, you get the overview. Schema verbs use the
two-word name: `bottle help schema add`, not
`bottle help add`.

The page is prose, not a TSV table. `--db` is accepted and
ignored; help does not open the store. An unknown topic
exits 2.
