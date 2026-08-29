# Schema

bottle ships with no schemas. `schema add` reads a YAML spec
and stores each field as a catalog row, then creates a
sqlite table for that type. YAML is an input format. It is
not stored.

## One table per schema

Each type has its own columns. `schema add nutrition.meal`
creates `entry_nutrition_meal`. `_` in a schema name is `.`
(`nutrition_meal` is `nutrition.meal`). The stored name
uses dots. The table name is `entry_` plus underscores.

```sql
CREATE TABLE schemas (
  name     TEXT PRIMARY KEY,
  retired  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE schema_fields (
  schema   TEXT NOT NULL,
  position INTEGER NOT NULL,
  name     TEXT NOT NULL,
  kind     TEXT NOT NULL,
  required INTEGER NOT NULL,
  PRIMARY KEY (schema, name),
  UNIQUE (schema, position)
);

CREATE TABLE schema_enum_values (
  schema   TEXT NOT NULL,
  field    TEXT NOT NULL,
  position INTEGER NOT NULL,
  value    TEXT NOT NULL,
  PRIMARY KEY (schema, field, value),
  UNIQUE (schema, field, position)
);
```

`kind` is `text`, `number`, or `enum`. `schema add-field`,
`schema add-value`, and `schema rename-field` change
catalog rows. `retired`
blocks new `log`s. Existing entries stay readable. sqlite
stores `retired` and `ignored` as `0`/`1`. TSV prints
`true`/`false`.

`schema show` prints the current fields as TSV.
`--yaml` prints the field list as YAML, the same form
`schema add --file` accepts.

The file is opened with WAL and a 5000 ms busy timeout.
See [tech-stack.md](tech-stack.md).

## Columns

Every data table starts with bookkeeping columns, then the
fields from the spec, in spec order. Links are not columns.
They live in a side table.

```sql
CREATE TABLE entry_fitness_set (
  id       INTEGER PRIMARY KEY,
  at       TEXT NOT NULL,
  agent    TEXT,
  ignored  INTEGER NOT NULL DEFAULT 0,
  movement TEXT NOT NULL,
  reps     TEXT NOT NULL,
  load     TEXT,
  unit     TEXT,
  volume   TEXT
);
```

`at` is the event instant in UTC. See [time.md](time.md).
Numbers are `TEXT` decimals. Enums are `TEXT` checked on
write.

`id` is per table. `entry_fitness_set.id = 7` is not
`entry_fitness_session.id = 7`.

## Links

An entry may point at other existing entries. Each pointer
has a name you choose at write time (`session`, `project`,
`parent`). Names are not declared in the YAML. The target
is any existing entry, any schema. Ignored targets still
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

One name, one target, per entry. Several different names
on the same entry are fine. `--link session=fitness.session/7`
replaces that name if it was already set. To point the
same name at two entries, log two entries.

```
bottle log fitness.set --link session=fitness.session/7 \
  movement=squat reps=8 load=24 unit=kg
```

`--link session=fitness.session/7` lists entries with
that link. `sum --group session` groups by that target.

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

Types: `text`, `number`, `enum`. Allowed YAML keys on a
field are `name`, `type`, `required`, and `values`. Other
keys (`default`, `unit`, …) are rejected. `default` is
`schema add-field --default`, which backfills existing
rows. A field named `unit` is a declared field, not a spec
property.

- Unknown fields on `log` are rejected.
- Missing required fields are rejected.
- `text` may not contain tab or newline. Compare is
  case-sensitive.
- `number` is an integer or float in stored form (`1`,
  `1.10`), not `1e3`, `01`, or `+1`. stderr names the
  rule (plain number, no exponent, no plus, no leading
  zero). Only these can be summed.
- `enum` values are stored lowercase. Leading and trailing
  spaces are stripped. On write and `--where`, `Water`
  becomes `water` and must match a declared value.
  Declared values are trimmed and lowercased on
  `schema add` / `add-field` / `add-value`. Duplicates
  after fold are rejected. Tab, newline, and comma are
  rejected.
- There is no date field type. Time is `at`.
- Field names: `^[a-z][a-z0-9_]*$`.
- Schema names: one or more segments joined by `.`
  (`meal`, `nutrition.meal`, `fitness.strength.set`).
  Each segment is `^[a-z][a-z0-9]*$`. `_` is accepted as
  `.` (`nutrition_meal` is `nutrition.meal`). Empty
  segments are rejected (`meal.`, `.meal`, `foo..bar`,
  `foo__bar`). Dots are a namespace convention only.
- Link names use the field-name regex. A link name may not
  be a field on that schema, and may not be reserved.
- Reserved: `id`, `at`, `agent`, `ignored`, `links`.
  Link names also may not be `day`, `week`, `month`,
  `year` (`sum --group`).

## ignore and unignore

`ignore` keeps the entry and hides it from `ls`, `sum`,
`last`, and `today`. `get` still returns it. `unignore`
clears `ignored` so those queries see it again. Both are
idempotent. Neither clears links. `amend` does not clear
`ignored`.

## Changing a schema

`schema add-field` adds one field (`ALTER TABLE ADD COLUMN`).
Without `--default` the field is optional and old entries
are empty there. With `--default` the field becomes
required and old entries are backfilled. It fails if a
link on that schema already uses the new field name.

`schema add-value` appends one enum value. You may not
remove one.

`schema rename-field` renames one field (`ALTER TABLE
RENAME COLUMN`) and updates the catalog. Values stay.
It fails if the new name exists, the schema is retired,
or a link on that schema already uses the new name.

To drop a field or change a type: add a new schema, copy
the entries you want, `schema retire` the old name.

## retire and drop

`schema retire` blocks `log`. Reads still work.

`schema drop` drops the table and its entries, deletes
outbound links from those entries, then removes the
registry record. It fails if any inbound link references
those ids, ignored or not.

## Why not raw SQL

SQL is too powerful and too broad. Bots overthink it, and a
rogue one can corrupt the store. bottle is sqlite behind a
CLI that says what they may do. A human may open the file.
A bot gets the verbs.
