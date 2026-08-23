## get

Prints one entry by schema and id, including ignored
entries. `ls` hides ignored entries; `get` is how you
still see one.

```
bottle get <schema> <id>
```

Ids are per schema. `fitness.set` 7 is not
`fitness.session` 7, so the schema name is required.

Columns are the same as `ls`, plus `ignored` (`true` or
`false`). Exit 1 if that id does not exist on that
schema.
