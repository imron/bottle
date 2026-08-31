# mcp

## Name

mcp — run bottle as an MCP server

## Synopsis

```
bottle mcp
```

## Description

Runs bottle as an MCP server on stdio. The tools are the same commands as the
CLI. Use this when a bot should talk to bottle without a shell.

The schema name is an argument on tools that need one. `schema_add` takes the
YAML as `spec`, not a file path.

Each tool result is TSV with a header for entry commands, prose for `help`, YAML
for `schema_show` with `yaml`. A failed call is a tool error, not a TSV body.

## Tools

`help`, `schema_list`, `schema_show`, `schema_add`, `schema_add_field`,
`schema_add_value`, `schema_rename`, `schema_rename_field`, `schema_retire`,
`schema_drop`, `log`, `ls`, `get`, `sum`, `last`, `today`, `amend`, `ignore`,
`unignore`, `backup`.

`help` takes an optional `command` (`log`, `schema add`).

On `ls` / `sum` / `last` / `today`, `exclude` is a list of `{field, value}`. A
row drops if it matches any.

`log` takes `entries`, a list of objects. One entry is a one-element list. All
entries share one schema and run in one transaction: all succeed or none do. Put
`at`, `agent`, and `links` on the entry they belong to. Declared cells go in
`fields`, same as `amend`. `check` true validates every entry and returns `rows`
and the count. It does not write.

`links` is an object of name to `schema/id`. `unlink` is a list of names.

Field values are strings or JSON numbers. The number's text is what the CLI
would take. `log` and `amend` use that for cells in `fields` (`null` is empty).
`where` uses it for filters. Booleans and nested objects are rejected.

`backup` takes `path` on the machine running bottle.

## See also

overview, log, help
