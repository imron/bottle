# get

## Name

get — print one entry by schema and id

## Synopsis

```
bottle get <schema> <id>
```

## Description

Prints one entry, including ignored entries. `ls` hides
ignored entries; `get` is how you still see one.

Ids are per schema. `fitness.set` 7 is not
`fitness.session` 7, so the schema name is required.

## Output

Same columns as `ls`, plus `ignored` (`true` or `false`).

## Exit status

`0` ok. `1` not found.

## See also

ls, amend, ignore
