# schema list

## Name

schema list — list registered schemas

## Synopsis

```
bottle schema list
bottle schema ls
```

## Description

Lists every schema that has been registered, including
retired ones. `schema ls` is the same command.

An empty list prints only the header. That is normal:
bottle ships with no schemas. Add one with
`bottle schema add`, then list again.

Retired schemas stay on the list so you can still show
their fields and read their entries. To see the fields of
one name, use `bottle schema show <name>`.

## Output

TSV with columns `name` and `retired`, sorted
alphabetically. `retired` is `true` or `false`.

## See also

schema, schema show, schema add
