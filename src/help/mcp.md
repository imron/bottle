## mcp

### What

Runs bottle as an MCP server on stdio. The same verbs as
the CLI, as tools.

### Why

Some bots should not get a shell. The tool result is the
same TSV (or the same help prose) the CLI would print, so
there is one contract. A new schema must not require an
MCP rebuild: there is one `log` tool, not `log_<schema>`.

### How

```
bottle mcp
```

Tools: `schema_list`, `schema_show`, `schema_add`,
`schema_add_field`, `schema_add_value`, `schema_retire`,
`schema_drop`, `help`, `log`, `ls`, `get`, `sum`,
`last`, `today`, `amend`, `ignore`.

`help` takes an optional `command` (`log`, `schema add`).
`log` accepts `fields` (one entry) or `entries` (many, one
transaction). Do not send both. Shared `at`, `agent`, and
`links` apply to every entry unless an entry overrides
them.
`links` is an object of name to `schema/id`. `unlink` is
a list of names.

This process speaks MCP, not TSV, on the pipe. Each tool
result's body is still the CLI bytes.
