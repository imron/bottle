# MCP

MCP exposes the same verbs as the CLI, for bots that should
not get a shell.

## Shape

stdio. The same binary can serve it (`bottle mcp`). Tools:

`help`, `schema_list`, `schema_show`, `schema_add`,
`schema_add_field`, `schema_add_value`, `schema_retire`,
`schema_drop`, `log`, `ls`, `get`, `sum`, `last`,
`today`, `amend`, `ignore`

Arguments match the CLI. `help` takes an optional
`command` (`log`, `schema add`). `get`, `amend`, and
`ignore` take `schema` and `id`. On `log` / `amend`,
`agent` is who wrote the entry. On `ls` / `sum` / `last` /
`today`, `agent` filters that column. A link target is
the string `schema/id`. `links` is an object of name to
target. `unlink` is a list of names.

## One entry or many

`log` accepts `fields` (one entry) or `entries` (a list of
field objects). Do not send both. All `entries` share one
schema and run in one transaction: all succeed or none do.
Shared `at`, `agent`, and `links` apply to every entry
unless an entry overrides them.

Callers that have a compact form (for example `4x8x24`)
expand it before calling. bottle stores entries, not that
string.

The result is one TSV line of `id`, `at`, `links` per
entry.

## Output

The tool result is the same bytes the CLI would print: TSV
for entry commands, prose for `help`, YAML for
`schema_show` with `yaml`. Not a JSON array of objects.

Errors are tool errors. A failed call does not return a TSV
body.

## One log tool

There is no generated `log_<schema>` tool. A new schema
must not require an MCP rebuild. The schema name is an
argument. The registry is the type system.

## TSV

Lines are flat. A header plus values is enough. The MCP
layer does not wrap the table in an object.
