# Bottle

Bottle is a tiny ledger that gives autonomous bots the ability to log facts
against custom schemas and query those facts using a fixed set of commands.

It provides lightweight access to structured data.

## Why it exists

Bots are good at noticing facts but bad at keeping them. The usual method is
keeping notes in markdown files, but such notes lack consistent type information
and structure. You cannot sum these facts efficiently, you have no consistent
way to reason about them chronologically and two bots writing about the same
facts cannot safely share information without stepping on each other's toes.

Bottle corrects for these things.

Users specify a schema containing fields with simple types (text, number, enum).
Bots then log facts against those schemas and query those facts back in
meaningful ways.

## Why not markdown

A note is prose, bottle entries are structured data. Entries have types, ids,
dates, and declared numbers you can group and sum far more efficiently than
parsing prose.

## Why not SQL

SQL is amazing and powerful, and with the full expressive power of SQL at their
disposal bots tend to overthink and overcomplicate solutions.

Rogue bots with full SQL access can also accidentally corrupt your data store,
and "Sorry, I shouldn't have done that, that's on me" is poor consolation when a
bot messes up.

Bottle is there to provide constraint. A small verb set over structured data.

## Why not JSONL or CSV

An append-only file looks like a ledger until you need to link data, or ignore
data or you have a field with bad data, or you need to do something with the
data that the format makes difficult.

Bottle provides structure and validation for your data. It lets you link data,
group data and ignore data, and its output is tab separated fields making it
convenient to use in pipes with all the unix commands you know and love.

## Why not a specialized app

Your training app, bank app, calendar app and other apps all have unique
interfaces with different ways of accessing and storing data. This can make it
tricky and slow for bots to navigate - especially for repeated queries.

These apps can be the source of truth for a fact, and bottle then becomes the
convenient interface that other bots use to query data from these source in a
consistent and reliable manner.

One bot can speak to the specialized apps. Other bots just need to speak bottle.

# Install

Rust is required ([rustup](https://rustup.rs/)).

```
cargo build --release
```

The binary is `target/release/bottle`.

Compile with musl if you want a static binary that can be copied without needing
glibc:

```
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

# Commands

```
$ bottle --help

Usage: bottle [OPTIONS] <COMMAND>

Commands:
  help      Print the long explanation of a command
  mcp       Run bottle as an MCP server on stdio
  schema    Declare and change types of entry
  log       Write one entry of a registered schema
  ls        List entries of a schema [alias: list]
  get       Print one entry by schema and id
  sum       Total a number field
  last      Print the most recent entry of a schema
  today     List entries for the current civil day
  amend     Change an existing entry in place
  ignore    Hide an entry from lists and totals
  unignore  Show an ignored entry in lists and totals again
  backup    Copy the ledger to a sqlite file

Options:
      --db <DB>    Path to the database file
      --no-header  Hide the TSV header
  -h, --help       Print help
```

Run `bottle help <command>` to see detailed help for each command.
