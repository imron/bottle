# get

## Name

get — print one entry by schema and id

## Synopsis

```
bottle get <schema> <id>
```

## Description

Prints one entry by schema and id, including entries that
have been ignored. `ls` omits those; `get` still shows
them.

Ids are per schema. `fitness.set` 7 is not
`fitness.session` 7, so the schema name is required.

## Output

Columns: `id`, `at`, `links`, the schema's fields in
declaration order, `agent`, and `ignored` (`true` or
`false`).

## Exit status

`0` ok. `1` not found.

## See also

ls, amend, ignore, unignore
