## schema show

Prints the field list for one schema: names, types,
whether each is required, and enum values.

```
bottle schema show <name>
bottle schema show <name> --yaml
```

Default output is TSV, one line per field, in spec order:

```
name	type	required	values
when	enum	true	breakfast,snack,lunch,dinner,extra
what	text	true	
kcal	number	true	
```

`required` is `true` or `false`. `values` is
comma-separated for an enum, and empty for `text` and
`number`.

`--yaml` prints the stored spec, the same YAML that
`schema add --file` accepts. Use that when you want to
copy or round-trip a schema.

Exit 1 if the name is unknown. Retired schemas still
show; retiring blocks new logs, not this command.

Links are not fields. They are not in this output. See
`bottle help log` for how links work.
