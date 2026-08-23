## today

### What

`ls` for the current civil day in the host timezone.

### Why

Bots should not compute local midnight. DST days are 23
or 25 hours; a fixed offset is wrong. This command is
that window. It does not total. Run `sum` for a total.

### How

```
bottle today <schema> [--agent NAME] \
  [--where field=value]...
```

Same columns as `ls`. Same `--agent` / `--where` rules.
Ignored entries are omitted.
