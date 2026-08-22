# help

This file is the long help. `bottle help` prints the
overview. `bottle help log` prints the log page. Schema
verbs are `bottle help schema add`. The binary prints the
matching section, nothing else.

`--help` on any command is the short usage: flags and
argument names. `help` is the page a human or a bot should
read to understand the verb.

Output is prose, not TSV. Unknown topic: exit 2.

---

## overview

bottle is a store for events. A schema is a type. A row is
one fact. Bots register a schema, log rows, and query with
a small set of verbs. They do not invent tables or issue
SQL.

Nothing ships pre-registered. Add a schema, then log.

```
bottle [--db PATH] <command>
```

`--db` / `BOTTLE_DB` is the sqlite file. If unset: Linux
`$XDG_DATA_HOME/bottle/bottle.db` or
`~/.local/share/bottle/bottle.db`; macOS
`~/.config/bottle/bottle.db`. `BOTTLE_AGENT` is the default
`--agent` on write. Set it to the bot's name.

Commands that return rows print TSV: header always, even
for one row. Booleans print `true` or `false`. Errors go
to stderr, never mixed into a TSV body. `help` is prose.
`schema show --yaml` is YAML.

Exit codes: `0` ok, `2` usage, `1` anything else (unknown
schema, bad field, not found).

A link is a named pointer to an existing row:
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

---

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

---

## schema

### What

The `schema` verbs declare and change types: `list`,
`show`, `add`, `add-field`, `add-value`, `retire`,
`drop`.

### Why

A row is only meaningful if the fields are declared.
Unknown fields are rejected. `sum` only runs on declared
numbers. The schema is the type system. Bots do not
`CREATE TABLE`.

### How

```
bottle schema list
bottle schema show <name> [--yaml]
bottle schema add <name> --file spec.yaml
bottle schema add-field <name> --name <field> --type ...
bottle schema add-value <schema> --field <name> --value <v>
bottle schema retire <name>
bottle schema drop <name>
```

`bottle help schema add` (and the other names) for one
verb. Names are `family.kind` (`nutrition.meal`). sqlite
gets an underscore (`nutrition_meal`). The CLI keeps the
dot.

---

## schema list

### What

Lists registered schemas.

### Why

Nothing is built in. A bot has to see which types this
store already has before it logs or invents a name.

### How

```
bottle schema list
```

TSV: `name`, `retired`. Alpha order. `retired` is `true`
or `false`. Retired schemas stay on the list so reads
still have a name. Field lists are `schema show`.

---

## schema show

### What

Prints the current field spec for one schema.

### Why

`schema list` is names only. The spec lives in sqlite, and
a bot may not open the file. After `add-field` or
`add-value`, or when another bot registered the type, this
is how you see what `log` will accept. Links are not in
the spec; they are not shown.

### How

```
bottle schema show <name>
bottle schema show <name> --yaml
```

Default TSV, one row per field, spec order: `name`,
`type`, `required`, `values`. `required` is `true` or
`false`. `values` is comma-separated for `enum`, empty
otherwise.

`--yaml` prints the stored spec, the same YAML
`schema add --file` accepts.

Exit 1 if missing. Retired schemas still show.

```
bottle schema show nutrition.meal
```

```
name	type	required	values
when	enum	true	breakfast,snack,lunch,dinner,extra
what	text	true	
kcal	number	true	
protein	number	true	
carbs	number	true	
fat	number	false	
```

---

## schema add

### What

Registers a type from a YAML file and creates its table.

### Why

A log with undeclared fields is a note. Adding the schema
is what makes later `sum` and `--where` safe. The YAML is
the field list, not the event log.

### How

```
bottle schema add <name> --file spec.yaml
```

`<name>` must be `family.kind` and must match
`^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*$`. Fails if the name
exists.

YAML:

```yaml
fields:
  - name: amount
    type: number
    required: true
  - name: kind
    type: enum
    required: true
    values: [in, out]
  - name: note
    type: text
    required: false
```

Types: `text`, `number`, `enum`. Enum values are stored
lowercase (`Water` → `water`). Text compare is
case-sensitive. Field names:
`^[a-z][a-z0-9_]*$`. Reserved field names: `id`, `at`,
`agent`, `ignored`, `links`. No date field; time is `at`.
Links are not declared here; they are set on `log`.

---

## schema add-field

### What

Adds one column to an existing schema.

### Why

The store is not a general migrator. You may grow a type
by one field. You may not rename, drop, or change a type
in place. For those, add a new schema, copy the rows you
want, retire the old name.

### How

```
bottle schema add-field <name> --name <field> \
  --type text|number|enum [--values a,b] [--default N]
```

Without `--default` the field is optional and old rows are
empty there. With `--default` the field is required and
old rows are backfilled. `--values` is required for
`enum`. Values are stored lowercase. Fails if the field
exists, the schema is retired, or two values fold to the
same lowercase string.

---

## schema add-value

### What

Appends one value to an enum field.

### Why

Enums are closed on write. A new real-world case (a new
`channel`, a new `kind`) should not force a new schema.
Removing a value would make old rows invalid, so that is
not offered. To drop a value, add a new schema and copy
rows.

### How

```
bottle schema add-value <schema> --field <name> \
  --value <v>
```

Fails if the field is not an enum, the folded value
already exists, or the schema is retired. The value is
stored lowercase.

---

## schema retire

### What

Blocks new `log`s on a schema. Existing rows stay
readable.

### Why

A type you no longer write should not disappear. `ls`,
`get`, and `sum` still work. Use this when you have
replaced the schema and want the old name kept as
history. Idempotent.

### How

```
bottle schema retire <name>
```

`schema add-field` and `schema add-value` also fail on a
retired schema. `amend`, `ignore`, and reads do not.

---

## schema drop

### What

Deletes a schema, its table, its rows, and outbound links
from those rows.

### Why

Retire keeps history. Drop is gone. It is the only way to
remove a type. It is refused if anything still points at
those ids, so a drop cannot leave dangling links.

### How

```
bottle schema drop <name>
```

Fails if any link in any table points at those ids,
ignored or not. `ignore` does not clear links. Unlink
first:

```
bottle amend fitness.set 2 --unlink session
bottle schema drop fitness.session
```

---

## log

### What

Writes one row of a registered schema.

### Why

This is the write path. One event, declared fields, an
optional instant, optional named links. A correction later
is `amend`, not a second sentence in a note. Many rows in
one transaction: MCP `rows`, not a shell loop that can
stop halfway.

### How

```
bottle log <schema> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [field=value ...]
```

Required fields must be present. Unknown fields are
rejected. `text` may not contain tab or newline, and
compare is case-sensitive. `enum` values are folded
lowercase and must match a declared value (`Water` →
`water`). Fails if the schema is retired.

`--at` defaults to now. A date-only `--at` is an error
(that is a query bound, not an instant). Accepted
instants: `...Z`, `...+10:00`, or naive `YYYY-MM-DDTHH:MM:SS`
in the host zone. Always stored as UTC seconds `Z`.
Printed local, with offset, never `Z`.

`--agent` defaults to `BOTTLE_AGENT`. Empty if unset.

`--link session=fitness.session/1` points this row at
another existing row. Repeat `--link` for different names.
A name once per command. One name, one target, per row.
The target must exist (ignored rows still exist). A link
name uses the field-name regex, must not be a field on
this schema, and must not be reserved (`id`, `at`,
`agent`, `ignored`, `links`, `day`, `week`, `month`,
`year`).

Prints `id`, `at`, `links`:

```
bottle log crm.touch who=ada channel=email
```

```
id	at	links
1	2026-08-22T08:14:00+10:00
```

```
bottle log fitness.set --link session=fitness.session/1 \
  movement=squat reps=8 load=24 unit=kg
```

```
id	at	links
1	2026-08-22T08:14:00+10:00	session=fitness.session/1
```

---

## ls

### What

Lists rows of a schema, oldest first.

### Why

The read path for "what was logged." Not SQL. Filters are
a closed set: time window, agent, field equality, link
equality. Ignored rows are omitted unless you ask for
them.

### How

```
bottle ls <schema> [--from DATE|TIME] [--to DATE|TIME] \
  [--agent NAME] [--where field=value]... \
  [--include-ignored]
```

Columns: `id`, `at`, `links`, schema fields in spec
order, `agent`. `ignored` only with `--include-ignored`.
Order: oldest `at`, then `id`. The schema name is not
repeated on each line.

`--from` / `--to` as a date are that civil day in the host
zone, inclusive on both ends. As a full timestamp, an
instant bound. `--from` alone has no end. `--to` alone
has no start.

`--agent` filters the bookkeeping column (who wrote the
row). On `log` / `amend` the same flag sets it.

`--where` may repeat (AND). If the name is a declared
field, it filters that field (`enum` folded lowercase,
`text` exact). Otherwise it is a link name and the value
must be `schema/id`. `--where` on `id`, `at`, `agent`,
`ignored`, or `links` is an error.

```
bottle ls fitness.set --where session=fitness.session/1
bottle ls money.txn --from 2026-08-01 --to 2026-08-31 \
  --where kind=out
```

The `links` cell is space-separated `name=schema/id`
pairs, sorted by name. Empty means no links.

---

## get

### What

Prints one row by schema and id, including ignored.

### Why

Ids are per table. `7` is not a store-wide id, so the
schema is required. `ls` hides ignored rows; `get` is how
you still see one.

### How

```
bottle get <schema> <id>
```

Same columns as `ls`, plus `ignored`. Exit 1 if missing.

---

## sum

### What

Totals a declared number field.

### Why

This is why the store is not markdown. Only fields
declared `number` can be summed. Bots should not multiply
in their head when the type already says the field is
additive. `today` does not print a total; run `sum`.

### How

```
bottle sum <schema> <field> [--from DATE|TIME] \
  [--to DATE|TIME] [--agent NAME] \
  [--where field=value]... \
  [--group day|week|month|year|<link>]
```

Fails if `<field>` is not a declared number. `--from`,
`--to`, `--agent`, `--where`: same as `ls`. Ignored rows
are omitted.

With no `--group`: columns `field`, `value`. An empty set
is one row, `value` `0`.

`--group day|week|month|year` uses the host zone:

- `day` -- `YYYY-MM-DD`
- `week` -- ISO `YYYY-Www`
- `month` -- `YYYY-MM`
- `year` -- `YYYY`

Any other `--group` name is a link name. The group column
is that name; the cell is `schema/id`. Rows with no such
link are one group with an empty cell. An empty set with
`--group` prints the header and no rows.

```
bottle sum nutrition.meal protein --from 2026-08-16 \
  --to 2026-08-22 --group day
bottle sum work.hours hours --where project=work.project/1
bottle sum work.hours hours --group project
```

Numbers print without trailing zeros (`49` not `49.0`).

---

## last

### What

Prints the newest row of a schema, optionally filtered.

### Why

"When did I last speak to ada" is one row, not a list.
Newest `at`, then highest `id` if two share an instant.

### How

```
bottle last <schema> [--agent NAME] \
  [--where field=value]...
```

Same columns as `ls`. Same `--agent` / `--where` rules.
Ignored rows are omitted. Exit 1 if none.

```
bottle last crm.touch --where who=ada
```

---

## today

### What

`ls` for the current civil day in the host timezone.

### Why

Bots should not compute local midnight. DST days are 23
or 25 hours; a fixed offset is wrong. This command is
that window. It does not total. Run `sum` for a total.

### How

```
bottle today <schema> [--agent NAME] \
  [--where field=value]...
```

Same columns as `ls`. Same `--agent` / `--where` rules.
Ignored rows are omitted.

---

## amend

### What

Changes a row in place: fields, `at`, `agent`, links.

### Why

A correction is an edit, not another paragraph. The id
stays. `ignore` is for a row that should vanish from
lists; `amend` is for a row that should stay, fixed.
Does not clear `ignored`.

### How

```
bottle amend <schema> <id> [--at TIME] [--agent NAME] \
  [--link name=SCHEMA/ID]... [--unlink name]... \
  [field=value ...]
```

At least one of `--at`, `--agent`, `--link`, `--unlink`,
or a `field=` is required. `--link` sets or replaces that
name's target. The target must exist. `--unlink name`
removes that name. Idempotent if the name is already
absent (still prints the row). `--link` and `--unlink` of
the same name in one command is an error. Date-only
`--at` is an error. Prints `id`, `at`, `links`. Exit 1 if
missing.

```
bottle amend nutrition.fluid 1 ml=375
bottle amend fitness.set 2 --link session=fitness.session/9
bottle amend fitness.set 2 --unlink session
```

---

## ignore

### What

Hides a row from `ls`, `sum`, `last`, and `today`. `get`
still returns it.

### Why

A bad row should not be deleted: drop is for types, and
there is no un-ignore. The fact is kept. If you need it
visible again, log a new row. `ignore` does not clear
links; inbound links still block `schema drop`.

### How

```
bottle ignore <schema> <id>
```

Sets `ignored`. Idempotent. Prints `id`, `at`. Exit 1 if
missing.

---

## mcp

### What

Runs bottle as an MCP server on stdio. The same verbs as
the CLI, as tools.

### Why

Some bots should not get a shell. The tool result is the
same TSV (or the same help prose) the CLI would print, so
there is one contract. A new schema must not require an
MCP rebuild: there is one `log` tool, not `log_<schema>`.

### How

```
bottle mcp
```

Tools: `schema_list`, `schema_show`, `schema_add`,
`schema_add_field`, `schema_add_value`, `schema_retire`,
`schema_drop`, `help`, `log`, `ls`, `get`, `sum`,
`last`, `today`, `amend`, `ignore`.

`help` takes an optional `command` (`log`, `schema add`).
`log` accepts `fields` (one row) or `rows` (many, one
transaction). Do not send both. Shared `at`, `agent`, and
`links` apply to every row unless a row overrides them.
`links` is an object of name to `schema/id`. `unlink` is
a list of names.

This process speaks MCP, not TSV, on the pipe. Each tool
result's body is still the CLI bytes.
