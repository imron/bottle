## schema list

Lists every schema that has been registered in this store,
including retired ones.

```
bottle schema list
```

Output is TSV with columns `name` and `retired`, sorted
alphabetically. `retired` is `true` or `false`.

An empty store prints only the header. That is normal:
bottle ships with no schemas. Add one with
`bottle schema add`, then list again.

Retired schemas stay on the list so you can still `show`,
`ls`, and `get` them. To see the fields of one name, use
`bottle schema show`.
