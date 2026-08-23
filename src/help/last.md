## last

Prints the newest entry of a schema, optionally filtered.
Use this for questions like "when did I last speak to
ada" — one result, not a list.

```
bottle last <schema> [--agent NAME] \
  [--where field=value]...
```

Newest `at` wins. If two entries share an instant, the
highest `id` wins.

Columns are the same as `ls`. `--agent` and `--where`
follow the same rules as `ls`. Ignored entries are
omitted. Exit 1 if nothing matches.

```
bottle last crm.touch --where who=ada
```
