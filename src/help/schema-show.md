## schema show

### What

Prints the current field spec for one schema.

### Why

`schema list` is names only. The spec lives in sqlite, and
a bot may not open the file. After `add-field` or
`add-value`, or when another bot registered the type, this
is how you see what `log` will accept. Links are not in
the spec; they are not shown.

### How

```
bottle schema show <name>
bottle schema show <name> --yaml
```

Default TSV, one line per field, spec order: `name`,
`type`, `required`, `values`. `required` is `true` or
`false`. `values` is comma-separated for `enum`, empty
otherwise.

`--yaml` prints the stored spec, the same YAML
`schema add --file` accepts.

Exit 1 if missing. Retired schemas still show.

```
bottle schema show nutrition.meal
```

```
name	type	required	values
when	enum	true	breakfast,snack,lunch,dinner,extra
what	text	true	
kcal	number	true	
protein	number	true	
carbs	number	true	
fat	number	false	
```
