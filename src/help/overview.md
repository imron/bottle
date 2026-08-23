## overview

bottle is a store for events. A schema is a type. An entry
is one fact. Bots register a schema, log entries, and
query with a small set of verbs. They do not invent tables
or issue SQL.

Nothing ships pre-registered. Add a schema, then log.

```
bottle [--db PATH] <command>
```

`--db` / `BOTTLE_DB` is the sqlite file. If unset: Linux
`$XDG_DATA_HOME/bottle/bottle.db` or
`~/.local/share/bottle/bottle.db`; macOS
`~/.config/bottle/bottle.db`. `BOTTLE_AGENT` is the default
`--agent` on write. Set it to the bot's name.

Commands that return entries print TSV: header always,
even for one line. Booleans print `true` or `false`. Errors go
to stderr, never mixed into a TSV body. `help` is prose.
`schema show --yaml` is YAML.

Exit codes: `0` ok, `2` usage, `1` anything else (unknown
schema, bad field, not found).

A link is a named pointer to an existing entry:
`--link session=fitness.session/1`. It is not a tag. See
`bottle help log` and `bottle help ls`.

Time is stored UTC and printed in the host timezone. Date
inputs are civil days in that zone. See `bottle help log`
and `bottle help ls`.

```
bottle help <command>
```

Topics: `help`, `schema`, `schema list`, `schema show`,
`schema add`, `schema add-field`, `schema add-value`,
`schema retire`, `schema drop`, `log`, `ls`, `get`,
`sum`, `last`, `today`, `amend`, `ignore`, `mcp`.
