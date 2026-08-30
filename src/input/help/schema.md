# schema

## Name

schema — declare and change types of entry

## Synopsis

```
bottle schema list
bottle schema show <name> [--yaml]
bottle schema add <name> --file spec.yaml
bottle schema add <name> --file -
bottle schema add-field <name> --name <field> --type ...
bottle schema add-value <schema> --field <name> --value <v>
bottle schema rename <old> <new>
bottle schema rename-field <schema> --from <old> --to <new>
bottle schema retire <name>
bottle schema drop <name>
```

## Description

A schema is a named type of entry: what fields it has,
which are required, and which values an enum may take.
bottle ships with none. You add a schema, then you can
`log` facts of that type.

The name is one or more lowercase identifiers separated by
dots, for example `meal`, `nutrition.meal`, or
`fitness.strength.set`. Dots are a namespace convention,
not a requirement.

`list` and `show` are reads. `add` creates a type.
`add-field` and `add-value` grow a type. `rename` renames
a schema and rewrites `old/id` links. `rename-field`
renames a field. You cannot drop a field or change its
type. `retire` blocks new logs but keeps existing entries.
`drop` deletes the type and its entries.

Unknown fields on `log` are rejected. `sum` only runs on
fields declared as `number`. Links are not part of the
YAML; you attach them when you log.

## See also

schema list, schema show, schema add, schema add-field,
schema add-value, schema rename, schema rename-field,
schema retire, schema drop, log
