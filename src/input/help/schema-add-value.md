# schema add-value

## Name

schema add-value — append one value to an enum field

## Synopsis

```
bottle schema add-value <schema> --field <name> \
  --value <v>
```

## Description

Adds one allowed value to an enum field. `log` rejects
anything not on the list. Use this when you need a new
value (`channel`, `kind`, and so on) without replacing the
schema.

The value is stored lowercase (`Brunch` becomes `brunch`).

You cannot remove a value. Removing one would make old
entries invalid. To drop a value, add a new schema and
copy the entries you want to keep.

## Options

`--field <name>` — an enum field on the schema.

`--value <v>` — the value to append.

## Exit status

`0` ok. `1` field is not an enum, value already exists
after fold, or schema is retired.

## See also

schema add, schema add-field, log
