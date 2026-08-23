## schema list

### What

Lists registered schemas.

### Why

Nothing is built in. A bot has to see which types this
store already has before it logs or invents a name.

### How

```
bottle schema list
```

TSV: `name`, `retired`. Alpha order. `retired` is `true`
or `false`. Retired schemas stay on the list so reads
still have a name. Field lists are `schema show`.
