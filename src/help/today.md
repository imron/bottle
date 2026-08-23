## today

Lists entries of a schema whose `at` falls on the current
civil day in the timezone of this machine. Same columns
as `ls`. It does not print a total; run `sum` for that.

```
bottle today <schema> [--agent NAME] \
  [--where field=value]...
```

The window is local midnight through the next local
midnight. DST days are 23 or 25 hours; bottle uses the
zone database, not a fixed offset such as "+10 hours".

`--agent` and `--where` follow the same rules as `ls`.
Ignored entries are omitted.
