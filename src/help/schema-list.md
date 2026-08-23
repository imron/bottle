# schema list

## Name

schema list — list registered schemas

## Synopsis

```
bottle schema list
```

## Description

Lists every schema that has been registered in this store,
including retired ones.

An empty store prints only the header. That is normal:
bottle ships with no schemas. Add one with `schema add`,
then list again.

Retired schemas stay on the list so you can still `show`,
`ls`, and `get` them. To see the fields of one name, use
`schema show`.

## Output

TSV with columns `name` and `retired`, sorted
alphabetically. `retired` is `true` or `false`.

## See also

schema, schema show, schema add
