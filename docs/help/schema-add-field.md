## schema add-field

### What

Adds one column to an existing schema.

### Why

The store is not a general migrator. You may grow a type
by one field. You may not rename, drop, or change a type
in place. For those, add a new schema, copy the entries
you want, retire the old name.

### How

```
bottle schema add-field <name> --name <field> \
  --type text|number|enum [--values a,b] [--default N]
```

Without `--default` the field is optional and old entries
are empty there. With `--default` the field is required and
old entries are backfilled. `--values` is required for
`enum`. Values are stored lowercase. Fails if the field
exists, the schema is retired, or two values fold to the
same lowercase string.
