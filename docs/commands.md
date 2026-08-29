# Commands

```
bottle [--db PATH] <command>
```

Output is TSV: one table, header always, even for one line.
`help` is prose, not TSV. `schema show --yaml` is the spec
YAML. Errors go to stderr and never share stdout with a
TSV body.

Exit codes: `0` ok, `2` usage, `1` anything else (unknown
schema, bad field, not found).

Numbers print as logged (`86.50` stays `86.50`; `49` stays
`49`). Booleans print `true` or `false`
(`retired`, `required`, `ignored`). Empty optional fields
are empty cells. `ls`, `today`, and `last` do not repeat
the schema name on each line.

Ignored entries are omitted from `ls`, `sum`, `last`, and
`today`. `get` still returns them. `unignore` clears
`ignored` so those queries see the entry again.

A link in flags and TSV is `schema/id`. The `links` cell is
space-separated `name=schema/id` pairs, sorted by name.
Empty means no links.

`--help` on any command is the short usage. `bottle help`
is the long page (what, why, how) for humans and bots.
See [src/input/help/](../src/input/help/).

## help

```
bottle help
bottle help <command>
```

Prints the matching page from [src/input/help/](../src/input/help/).
Overview if no command. Schema verbs are
`bottle help schema add`.
`--db` is accepted and ignored. Unknown topic: exit 2.

## schema list

```
bottle schema list
```

`schema ls` is the same command. TSV: `name`, `retired`.
Alpha order. `retired` is `true` or `false`.

## schema show

```
bottle schema show <name> [--yaml]
```

Default TSV, one line per field, spec order: `name`,
`type`, `required`, `values`. `required` is `true` or
`false`. `values` is comma-separated for `enum`, empty
otherwise.
Links are not fields; they are omitted. `--yaml` prints
the field list as YAML, the same form `schema add --file`
accepts. Exit 1 if missing. Retired schemas still show.

## schema add

```
bottle schema add <name> --file spec.yaml
```

Fails if `name` exists or is not one or more ident
segments joined by dots (no empty segments). Writes the
spec into the registry and creates the table.

## schema add-field

```
bottle schema add-field <name> --name <field> \
  --type text|number|enum [--values a,b] [--default N]
```

Adds one field. Optional unless `--default` is set. Then
the field is required and old entries are backfilled.
`--values` is required for `enum`. Leading and trailing
spaces are stripped. Values are stored lowercase. Fails
if the field exists, the schema is retired, the name is
already used as a link on that schema, or two values
fold to the same lowercase string.

## schema add-value

```
bottle schema add-value <schema> --field <name> \
  --value <v>
```

Appends one value to an enum, stored lowercase. Fails if
the field is not an enum, the folded value already exists,
or the schema is retired. You may not remove a value. To
drop one, add a new schema and copy entries.

## schema rename-field

```
bottle schema rename-field <schema> --from <old> --to <new>
```

Renames one field. Existing entries keep their values.
Type, required, enum values, and position stay the same.
Fails if `--from` is unknown, `--to` already exists, the
schema is retired, or `--to` is already used as a link on
that schema. `--from` and `--to` must differ.

## schema retire

```
bottle schema retire <name>
```

`log` fails. Reads still work. Idempotent.

## schema drop

```
bottle schema drop <name>
```

Drops the table and its entries, then outbound links from
those entries. Fails if any link in any table points at
those
ids, ignored or not. `amend --unlink` those links first.
`ignore` does not clear them.

## log

```
bottle log <schema> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [field=value ...]
```

`--at` defaults to now. A date-only `--at` is an error.
See [time.md](time.md). `--agent` defaults to
`BOTTLE_AGENT`, or `bottle` if that is unset. Fails if
the schema is retired. `--link`
may repeat with different names. A name once per command.
The target entry must exist. Prints `id`, `at`, `links`.

```
bottle log crm.touch who=ada channel=email
```

```
id	at	links
1	2026-08-22T08:14:00+10:00
```

The offset is the host zone at that instant. For many
entries in one transaction, `bottle log SCHEMA --file
rows.tsv` (or `--file -` for stdin). TSV only. Header
required. Schema is on the command. `--at`, `--agent`,
`--link`, and `field=value` default every row; a TSV cell
wins. Missing `at` is now. Missing `agent` is
`BOTTLE_AGENT`. All succeed or none do. Same as MCP `log`
`entries`. See [mcp.md](mcp.md).

## ls

```
bottle ls <schema> [--from DATE|TIME] [--to DATE|TIME] \
  [--agent NAME] [--where field=value]... \
  [--link name=SCHEMA/ID]... [--include-ignored]
```

`list` is the same command.

Columns: `id`, `at`, `links`, schema fields in spec
order, `agent`. `ignored` only with `--include-ignored`.
`--agent` filters that bookkeeping column. `--where` may
repeat (AND) and must name a declared field (`enum`
values folded lowercase; `text` exact). `--link` filters
by a named pointer; the value is `schema/id`. `--where`
on reserved names (`id`, `at`, `agent`, `ignored`,
`links`) is an error; use `--agent`, `get`, or
`--from` / `--to`. Order: oldest `at`, then `id`.

## get

```
bottle get <schema> <id>
```

Same field columns as `ls`, plus `ignored`. One entry,
including ignored. Exit 1 if missing. Schema is required
because ids are per table.

## sum

```
bottle sum <schema> <field> [--from DATE|TIME] \
  [--to DATE|TIME] [--agent NAME] \
  [--where field=value]... \
  [--link name=SCHEMA/ID]... \
  [--group day|week|month|year|<link>]
```

`<field>` must be a declared number. With no `--group`:
`field`, `value`. Time groups use the host zone:

- `day` -- `YYYY-MM-DD`
- `week` -- ISO `YYYY-Www`
- `month` -- `YYYY-MM`
- `year` -- `YYYY`

Any other `--group` name is a link name. The group column
is that name; the cell is `schema/id`. Entries with no
such link are one group with an empty cell.

An empty set prints `value` `0` (one line, or no group
lines).

## last

```
bottle last <schema> [--agent NAME] \
  [--where field=value]... [--link name=SCHEMA/ID]...
```

Newest `at`, then highest `id`. Same columns as `ls`.
Exit 1 if none.

## today

```
bottle today <schema> [--agent NAME] \
  [--where field=value]... [--link name=SCHEMA/ID]...
```

`ls` for the current host civil day. No totals. Run `sum`.

## amend

```
bottle amend <schema> <id> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [--unlink name]... \
  [field=value ...]
```

At least one of `--at`, `--agent`, `--link`, `--unlink`,
or a `field=` is required. `--link` sets or replaces that
name's target. `--unlink name` removes that name.
Idempotent if the name is already absent (still prints
the entry). `--link` and `--unlink` of the same name in one
command is an error. Prints `id`, `at`, `links`. Exit 1
if missing. Does not clear `ignored`.

## ignore

```
bottle ignore <schema> <id>
```

Sets `ignored`. Idempotent. Prints `id`, `at`. Exit 1 if
missing. Does not clear links. `unignore` is the inverse.

## unignore

```
bottle unignore <schema> <id>
```

Clears `ignored`. The entry is again visible to `ls`,
`sum`, `last`, and `today`. Idempotent if it was not
ignored (still prints `id`, `at`). Prints `id`, `at`.
Exit 1 if missing. Does not change fields, `at`, `agent`,
or links.

## backup

```
bottle backup <path>
```

Writes a consistent copy of the database to `<path>`.
The live file is unchanged. The copy is one sqlite file
(no `-wal` or `-shm`). `<path>` must not already exist.
No output. Exit 1 if the path exists, its parent is
missing, or the copy fails. Point `--db` at the copy to
read it. There is no restore verb.
