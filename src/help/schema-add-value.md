## schema add-value

### What

Appends one value to an enum field.

### Why

Enums are closed on write. A new real-world case (a new
`channel`, a new `kind`) should not force a new schema.
Removing a value would make old entries invalid, so that
is not offered. To drop a value, add a new schema and copy
entries.

### How

```
bottle schema add-value <schema> --field <name> \
  --value <v>
```

Fails if the field is not an enum, the folded value
already exists, or the schema is retired. The value is
stored lowercase.
