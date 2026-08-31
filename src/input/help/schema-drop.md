# schema drop

## Name

schema drop — delete a schema and its entries

## Synopsis

```
bottle schema drop <name>
```

## Description

Deletes a schema, all of its entries, and every outbound link from those
entries. `schema retire` keeps the entries readable; `drop` does not.

It refuses if any entry, in any schema, still links at this type — ignored or
not. `ignore` does not clear links. Unlink them first.

## Exit status

`0` ok. `1` unknown schema or inbound links remain.

## Examples

```
bottle amend fitness.set 2 --unlink session
bottle schema drop fitness.session
```

## See also

schema retire, amend, ignore, unignore
