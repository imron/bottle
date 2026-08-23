# mcp

## Name

mcp — run bottle as an MCP server

## Synopsis

```
bottle mcp
```

## Description

Runs bottle as an MCP server on stdio. The tools are the
same verbs as the CLI. Use this when a bot should not get
a shell.

There is one `log` tool, not a generated `log_meal`. A new
schema does not require an MCP rebuild; the schema name is
an argument.

The process speaks MCP on the pipe. Each tool result's
body is the same bytes the CLI would print: TSV for entry
commands, prose for `help`, YAML for `schema_show` with
`yaml`. A failed call is a tool error, not a TSV body.

## Tools

`help`, `schema_list`, `schema_show`, `schema_add`,
`schema_add_field`, `schema_add_value`, `schema_retire`,
`schema_drop`, `log`, `ls`, `get`, `sum`, `last`,
`today`, `amend`, `ignore`.

`help` takes an optional `command` (`log`, `schema add`).

`log` accepts `fields` (one entry) or `entries` (a list of
field objects). Do not send both. All `entries` share one
schema and run in one transaction: all succeed or none do.
Shared `at`, `agent`, and `links` apply to every entry
unless that entry overrides them.

`links` is an object of name to `schema/id`. `unlink` is a
list of names.

## See also

overview, log, help
