# MCP

MCP exposes the same verbs as the CLI, for bots that should not get a shell.

## Shape

stdio. The same binary can serve it (`bottle mcp`). Tools:

`help`, `schema_list`, `schema_show`, `schema_add`, `schema_add_field`,
`schema_add_value`, `schema_rename`, `schema_rename_field`, `schema_retire`,
`schema_drop`, `log`, `ls`, `get`, `sum`, `last`, `today`, `amend`, `ignore`,
`unignore`, `backup`

Arguments match the CLI except `schema_add`, which takes the YAML as `spec`
rather than a file path. `help` takes an optional `command` (`log`,
`schema add`). `get`, `amend`, `ignore`, and `unignore` take `schema` and `id`.
`backup` takes `path` on the machine running bottle. On `log` / `amend`, `agent`
is who wrote the entry. On `ls` / `sum` / `last` / `today`, `agent` filters that
column. A link target is the string `schema/id`. `links` is an object of name to
target. `unlink` is a list of names. `exclude` on `ls` / `sum` / `last` /
`today` is a list of `{field, value}`; a row drops if it matches any.

## One entry or many

`log` takes `entries`, a list of objects. One entry is a one-element list. All
entries share one schema and run in one transaction: all succeed or none do. Put
`at`, `agent`, and `links` on the entry they belong to. Declared cells go in
`fields`, same as `amend`.

Callers that have a compact form (for example `4x8x24`) expand it before
calling. bottle stores entries, not that string.

The result is one TSV line of `id`, `at`, `links` per entry. `check` true
validates every entry and returns `rows` and the count. It does not write and
does not invent ids. A bad batch is the same tool error as a real log.

## Output

The tool result is TSV with a header for entry commands, prose for `help`, YAML
for `schema_show` with `yaml`. Not a JSON array of objects.

Errors are tool errors. A failed call does not return a TSV body.

## One log tool

There is no generated `log_<schema>` tool. A new schema must not require an MCP
rebuild. The schema name is an argument. The registry is the type system.

## TSV

Lines are flat. A header plus values is enough. The MCP layer does not wrap the
table in an object.
