# last

## Name

last — print the newest entry of a schema

## Synopsis

```
bottle last <schema> [--agent NAME] \
  [--where field=value]...
```

## Description

Prints the newest entry of a schema, optionally filtered.
Use this for questions like "when did I last speak to
ada" — one result, not a list.

Newest `at` wins. If two entries share an instant, the
highest `id` wins. Ignored entries are omitted.

## Options

`--agent`, `--where` — same rules as `ls`.

## Output

Same columns as `ls`.

## Exit status

`0` ok. `1` nothing matches.

## Examples

```
bottle last crm.touch --where who=ada
```

## See also

ls, get, today
