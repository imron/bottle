# today

## Name

today — list entries for the current civil day

## Synopsis

```
bottle today <schema> [--agent NAME] \
  [--where field=value]...
```

## Description

Lists entries of a schema whose `at` falls on the current
civil day in the timezone of this machine. Same columns as
`ls`. It does not print a total; run `sum` for that.

The window is local midnight through the next local
midnight. DST days are 23 or 25 hours; bottle uses the
zone database, not a fixed offset such as "+10 hours".

Ignored entries are omitted.

## Options

`--agent`, `--where` — same rules as `ls`.

## Output

Same columns as `ls`.

## See also

ls, sum, last
