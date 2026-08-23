## schema

### What

The `schema` verbs declare and change types: `list`,
`show`, `add`, `add-field`, `add-value`, `retire`,
`drop`.

### Why

An entry is only meaningful if the fields are declared.
Unknown fields are rejected. `sum` only runs on declared
numbers. The schema is the type system. Bots do not
`CREATE TABLE`.

### How

```
bottle schema list
bottle schema show <name> [--yaml]
bottle schema add <name> --file spec.yaml
bottle schema add-field <name> --name <field> --type ...
bottle schema add-value <schema> --field <name> --value <v>
bottle schema retire <name>
bottle schema drop <name>
```

`bottle help schema add` (and the other names) for one
verb. Names are `family.kind` (`nutrition.meal`). sqlite
gets an underscore (`nutrition_meal`). The CLI keeps the
dot.
