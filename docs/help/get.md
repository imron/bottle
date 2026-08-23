## get

### What

Prints one entry by schema and id, including ignored.

### Why

Ids are per table. `7` is not a store-wide id, so the
schema is required. `ls` hides ignored entries; `get` is how
you still see one.

### How

```
bottle get <schema> <id>
```

Same columns as `ls`, plus `ignored`. Exit 1 if missing.
