# backup

## Name

backup — copy the ledger to a sqlite file

## Synopsis

```
bottle backup <path>
```

## Description

Writes a consistent copy of the database to `<path>`.
The live file is unchanged. The copy is one sqlite file
(no `-wal` or `-shm`). Point `--db` at the copy to read
it, or replace a stopped bottle's file with the copy.

`<path>` must not already exist. Relative paths are from
the process working directory. On MCP, `path` is on the
machine running bottle, not the client.

Do not copy `bottle.db` while the process is open. Use
this command instead.

## Output

None.

## Exit status

`0` ok. `1` the path exists, its parent is missing, or
the copy fails.

## See also

overview, mcp
