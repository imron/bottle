# schema show

## Name

schema show — print the field list of a schema

## Synopsis

```
bottle schema show <name>
bottle schema show <name> --yaml
```

## Description

Prints the field list for one schema: names, types, whether each is required,
and enum values.

Retired schemas still show. Links are not fields and are not in this output.
Pass them with `--link` on `log`.

## Options

`--yaml` — print the YAML field list, the same form that `schema add --file`
accepts.

## Output

Default is TSV, one line per field, in spec order:

```
name	type	required	values
when	enum	true	breakfast,snack,lunch,dinner,extra
what	text	true	
kcal	number	true	
```

`required` is `true` or `false`. `values` is comma-separated for an enum, and
empty for `text` and `number`.

## Exit status

`0` ok. `1` unknown schema.

## Examples

```
bottle schema show nutrition.meal
```

## See also

schema, schema list, schema add, log
