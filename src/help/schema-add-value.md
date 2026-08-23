## schema add-value

Adds one allowed value to an enum field. Enums are closed
on write: `log` will reject anything not on the list. When
the real world grows a new case (a new `channel`, a new
`kind`), append it here instead of creating a whole new
schema.

```
bottle schema add-value <schema> --field <name> \
  --value <v>
```

The value is stored lowercase (`Brunch` becomes `brunch`).
Fails if the field is not an enum, the folded value
already exists, or the schema is retired.

You cannot remove a value. Removing one would make old
entries invalid. To drop a value, add a new schema and
copy the entries you want to keep.
