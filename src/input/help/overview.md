# overview

## Name

bottle — a store for events

## Synopsis

```
bottle [--db PATH] [--no-header] <command>
```

## Description

bottle is a store for events. You declare a type (a
schema), log facts of that type (entries), and query them
with a fixed set of commands.

Nothing is built in. Add a schema, then log. Other
programs — banks, calendars, fitness apps — can stay the
source of a fact. You copy that fact into bottle so every
bot can query it here.

An entry has an id (per schema, not global), a time
(`at`), optional agent, optional named links to other
entries, and the fields you declared. A link
`--link session=fitness.session/1` means this entry's
`session` is entry 1 of `fitness.session`.

Times print in the timezone of the machine running bottle.
`--at` shape is the grain: a time is an instant, a date is
a day, `YYYY-MM` is a month. Printed `at` uses the same
shape.

## Files

`--db` or the environment variable `BOTTLE_DB` is the
database file. If neither is set:

- Linux: `$XDG_DATA_HOME/bottle/bottle.db`, else
  `~/.local/share/bottle/bottle.db`
- macOS: `~/.config/bottle/bottle.db`

WAL may also write `bottle.db-wal` and `bottle.db-shm`
next to the file. `backup` writes a single sqlite file.

## Environment

`BOTTLE_DB` — path to the database file.

`BOTTLE_AGENT` — default `--agent` on write. If unset, the
agent is `bottle`. Set it to the name of the bot that is
logging.

## Output

Most commands print a TSV table: a header line, then data
lines, even when there is only one result. `--no-header`
omits the header. Empty `ls` then prints nothing.
Booleans print `true` or `false`. Empty optional fields
are empty cells. Numbers print as logged (`86.50` stays
`86.50`). Errors go to stderr and never share stdout with
a table. `help` is prose. `schema show --yaml` is YAML.

## Exit status

`0` ok. `2` a usage mistake. `1` anything else (unknown
schema, bad field, not found).

## Commands

```
bottle help
bottle help log
bottle help schema add
```

Topics: `help`, `schema`, `schema list`, `schema show`,
`schema add`, `schema add-field`, `schema add-value`,
`schema rename`, `schema rename-field`, `schema retire`,
`schema drop`,
`log`, `ls`, `get`, `sum`, `last`, `today`, `amend`,
`ignore`, `unignore`, `backup`, `mcp`.
