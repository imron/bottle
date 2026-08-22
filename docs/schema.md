# Schema

bottle ships with no schemas. `schema add` writes a YAML spec
into a registry table and creates a sqlite table for that
type.

## One table per schema

Each type has its own columns. `schema add nutrition.meal`
creates `nutrition_meal`. The CLI and MCP keep the dotted
name. sqlite sees the underscore.

```sql
CREATE TABLE schemas (
  name     TEXT PRIMARY KEY,
  spec     TEXT NOT NULL,
  retired  INTEGER NOT NULL DEFAULT 0
);
```

`spec` is the YAML from `schema add`, updated by
`schema add-field` and `schema add-value`. `retired`
blocks new `log`s. Existing rows stay readable. sqlite
stores `retired` and `ignored` as `0`/`1`. TSV prints
`true`/`false`.

`schema show` prints the current fields as TSV.
`--yaml` prints `spec` as stored.

The file is opened with WAL and a 5000 ms busy timeout.
See [tech-stack.md](tech-stack.md).

## Columns

Every data table starts with bookkeeping columns, then the
fields from the spec, in spec order. Links are not columns.
They live in a side table.

```sql
CREATE TABLE fitness_set (
  id       INTEGER PRIMARY KEY,
  at       TEXT NOT NULL,
  agent    TEXT,
  ignored  INTEGER NOT NULL DEFAULT 0,
  movement TEXT NOT NULL,
  reps     REAL NOT NULL,
  load     REAL,
  unit     TEXT,
  volume   REAL
);
```

`at` is the event instant in UTC. See [time.md](time.md).
Numbers are `REAL`. Enums are `TEXT` checked on write.

`id` is per table. `fitness_set.id = 7` is not
`fitness_session.id = 7`.

## Links

A row may point at other existing rows. Each pointer has a
name you choose at write time (`session`, `project`,
`parent`). Names are not declared in the YAML. The target
is any existing row, any schema. Ignored targets still
count as existing.

```sql
CREATE TABLE links (
  from_schema TEXT NOT NULL,
  from_id     INTEGER NOT NULL,
  name        TEXT NOT NULL,
  to_schema   TEXT NOT NULL,
  to_id       INTEGER NOT NULL,
  PRIMARY KEY (from_schema, from_id, name)
);
```

One name, one target, per row. Several different names on
the same row are fine. `--link session=fitness.session/7`
replaces that name if it was already set. To point the
same name at two rows, log two rows.

```
bottle log fitness.set --link session=fitness.session/7 \
  movement=squat reps=8 load=24 unit=kg
```

`--where session=fitness.session/7` lists rows with that
link. `sum --group session` groups by that target.

On the CLI and in TSV the target is `schema/id`. The
`links` cell is space-separated `name=schema/id` pairs,
sorted by name:

```
project=work.project/1 session=fitness.session/7
```

TSV does not join the target's fields in. `get` needs a
schema because ids are per table:
`bottle get fitness.session 7`.

Ignored targets do not hide their children. `schema drop`
refuses if any link, from any table, still points at the
table being dropped. `ignore` does not clear links.
`amend --unlink` them first.

A link is a pointer, not a tag. `--link gym` is rejected.
`--link session=fitness.session/7` is stored.

## Field spec

```yaml
fields:
  - name: account
    type: text
    required: true
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

Types: `text`, `number`, `enum`.

- Unknown fields are rejected.
- Missing required fields are rejected.
- `text` may not contain tab or newline. Compare is
  case-sensitive.
- `number` is an integer or float. Only these can be summed.
- `enum` values are stored lowercase. On write and
  `--where`, `Water` becomes `water` and must match a
  declared value. Declared values are lowercased on
  `schema add` / `add-field` / `add-value`. Duplicates
  after fold are rejected.
- There is no date field type. Time is `at`.
- Field names: `^[a-z][a-z0-9_]*$`.
- Schema names: `^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*$`.
- Link names use the field-name regex. A link name may not
  be a field on that schema, and may not be reserved.
- Reserved: `id`, `at`, `agent`, `ignored`, `links`.
  Link names also may not be `day`, `week`, `month`,
  `year` (`sum --group`).

## ignore

`ignore` keeps the row and hides it from `ls`, `sum`,
`last`, and `today`. `get` still returns it. There is no
un-ignore. Log a new row if you need the fact back.

`amend` changes listed fields, `--at` / `--agent` if
given, and `--link` / `--unlink`. It does not clear
`ignored`.

## Changing a schema

`schema add-field` adds one field (`ALTER TABLE ADD COLUMN`).
Without `--default` the field is optional and old rows are
empty there. With `--default` the field becomes required and
old rows are backfilled.

`schema add-value` appends one enum value. You may not
remove one.

To rename a field, drop a field, or change a type: add a new
schema, copy the rows you want, `schema retire` the old
name.

## retire and drop

`schema retire` blocks `log`. Reads still work.

`schema drop` drops the table and its rows, deletes
outbound links from those rows, then removes the registry
entry. It fails if any inbound link references those ids,
ignored or not.

## Why not raw SQL

SQL is too powerful and too broad. Bots overthink it, and a
rogue one can corrupt the store. bottle is sqlite behind a
CLI that says what they may do. A human may open the file.
A bot gets the verbs.
