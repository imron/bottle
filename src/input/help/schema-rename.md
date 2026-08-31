# schema rename

## Name

schema rename — rename a schema

## Synopsis

```
bottle schema rename <old> <new>
```

## Description

Renames a schema in one transaction. Existing entries keep their ids. The
retired flag follows. Every `old/id` link, inbound or outbound, becomes
`new/id`. The old name is vacant; there is no dump and drop.

`new` must be a legal schema name and must not already exist. `old` must exist.
`old` and `new` must differ. A retired schema can still be renamed.

## Exit status

`0` ok. `1` old is missing, new exists, or new is not a legal schema name. `2`
old and new are the same name.

## Examples

```
bottle schema rename fitness.session fitness.workout
```

## See also

schema list, schema retire, schema drop, schema rename-field
