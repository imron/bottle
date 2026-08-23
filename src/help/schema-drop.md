## schema drop

Deletes a schema, all of its entries, and every outbound
link from those entries. This is gone, not hidden.
`schema retire` keeps history; `drop` does not.

```
bottle schema drop <name>
```

It refuses if any entry, in any schema, still links at
this type — ignored or not. `ignore` does not clear links.
Unlink them first:

```
bottle amend fitness.set 2 --unlink session
bottle schema drop fitness.session
```

Exit 1 if the name is unknown or inbound links remain.
