## schema drop

### What

Deletes a schema, its table, its entries, and outbound
links from those entries.

### Why

Retire keeps history. Drop is gone. It is the only way to
remove a type. It is refused if anything still points at
those ids, so a drop cannot leave dangling links.

### How

```
bottle schema drop <name>
```

Fails if any link in any table points at those ids,
ignored or not. `ignore` does not clear links. Unlink
first:

```
bottle amend fitness.set 2 --unlink session
bottle schema drop fitness.session
```
