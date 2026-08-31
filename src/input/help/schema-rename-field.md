# schema rename-field

## Name

schema rename-field — rename a field on an existing schema

## Synopsis

```
bottle schema rename-field <schema> --from <old> --to <new>
```

## Description

Renames one field. Existing entries keep their values under the new name. Type,
required, and enum values stay the same. Position in the field list stays the
same.

You cannot drop a field or change its type. For those, add a new schema, copy
the entries you still want, and `schema retire` the old name.

`--from` and `--to` must be different names. `--to` must not already exist on
the schema, and must not be in use as a link name on that schema.

## Options

`--from <old>` — the current field name.

`--to <new>` — the new field name (`^[a-z][a-z0-9_]*$`). Cannot be reserved:
`id`, `at`, `agent`, `ignored`, `links`, `grain`.

## Exit status

`0` ok. `1` unknown field, field exists, schema retired, reserved name, or the
new name is already used as a link. `2` from and to are the same name.

## Examples

```
bottle schema rename-field nutrition.meal --from kcal --to calories
```

## See also

schema add-field, schema add-value, schema show
