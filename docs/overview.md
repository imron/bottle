# Overview

bottle is a small store for bots. They register a schema, log
entries, and query with a fixed set of verbs: `ls`, `sum`,
`last`, `amend`, `ignore`, `unignore`, `backup`. Output is TSV.
The file is sqlite.

A bot may still read a specialized system (training log, bank,
calendar). It writes an entry here so every other bot can
query that fact the same way.

## Why it exists

Bots are good at noticing a fact and bad at keeping it. The
usual dump is a running note in markdown. That is not a type.
You cannot sum it. The date in the sentence may not be the
date of the event. Two bots writing about the same day do not
share an entry. A correction is another paragraph, not an
edit.

A schema is a name, a YAML field list, and a sqlite table. An
entry is one event. A total is a command.

## Why not markdown

A note is prose. bottle is entries. Entries have types, ids,
and declared numbers you can sum.

## Why not SQL

The file is sqlite. SQL is too powerful and too broad. Bots
overthink a wide interface, and a rogue one can corrupt the
store. The CLI is the constraint: a small verb set over that
file. A human may open sqlite. A bot may not.

## Why not JSONL or CSV

An append-only file looks like a ledger until you need a
link, an ignore, a bulk insert, or a declared number. CSV
also breaks on values that contain commas. YAML is for schema
files, not the event log.

## Why not a notes app

Another login, a UI bots are bad at, not a file on disk, not
TSV. That is a second brain. This is a store.

## Why not a specialized app

A training app, a bank, or a calendar can stay the source of
a fact. A bot reads them and writes an entry here. Other bots
should not have to learn each of those APIs to ask what
happened. They speak bottle.

## Goals

- Closed verbs. Bots do not invent tables or issue SQL.
- Declared schemas. Unknown fields are rejected. `sum` only
  runs on declared numbers.
- Nothing ships pre-registered. Adding a schema creates a
  table.
- An entry may carry named links to other existing entries.
  See [schema.md](schema.md).
- Instants stored as UTC, shown in the host timezone. See
  [time.md](time.md).
- TSV on stdout. Errors on stderr. `--no-header` omits the
  CLI header; MCP includes it. `help` is prose.
  `schema show --yaml` is YAML.
  See [commands.md](commands.md), [help.md](help.md), and
  [mcp.md](mcp.md).
- One static musl binary next to the db.

## Non-goals

- Not a notes app or journal.
- Not a domain product. Health, money, and CRM are schemas
  someone adds.
- Not a replacement for specialized apps. Those can stay
  sources of truth.
- Not a query language. No SQL for bots.
- Not a hosted service. The file lives on disk.
- Not a tag system. A link is a named pointer to an existing
  entry, not a free string.
- Not a general migrator. You can add an optional field, add
  an enum value, or retire the schema. See
  [schema.md](schema.md).

## Who uses it

Anyone running more than one bot that currently files facts
in prose, or that each talk to a different upstream system.
Anyone who wants MCP tools for "write this / total that"
without opening sqlite.

## Config

`--db` / `BOTTLE_DB` is the sqlite path. If unset:

- Linux: `$XDG_DATA_HOME/bottle/bottle.db`, else
  `~/.local/share/bottle/bottle.db`
- macOS: `~/.config/bottle/bottle.db`

`BOTTLE_AGENT` is the default `--agent` on write. If unset,
the agent is `bottle`. Bots should set this to their name.

Local time is the host computer's timezone. There is no
timezone flag.

## Docs

- [schema.md](schema.md) -- tables, links, migrate, drop
- [time.md](time.md) -- storage, input, output
- [commands.md](commands.md) -- verbs
- [help.md](help.md) -- where `bottle help` pages live
- [mcp.md](mcp.md) -- the same verbs as tools
- [examples.md](examples.md) -- sample schemas
- [tech-stack.md](tech-stack.md) -- language, crates, musl
