# Tech stack

bottle is a Rust crate. The library owns the store, the
verbs, TSV, help, and MCP. The binary is a thin wrapper:
parse argv, call the library, write stdout/stderr, exit.
One binary serves the CLI and the MCP server. Release
builds for Linux are statically linked with musl so the
host does not need a matching glibc or a sqlite library.

## Language

Rust. The crate edition is `2024`.

## Libraries

- **clap** (derive) -- CLI parsing.
- **rusqlite** with the `bundled` feature -- sqlite compiled
  into the binary. No system `libsqlite3`.
- **serde** and **serde_yaml** -- schema files.
- **jiff** -- parse and format timestamps, convert using the
  host IANA timezone, including DST.
- **rmcp** -- MCP over stdio.
- **directories** -- Linux XDG data dir. macOS is not the
  crate default; it is `~/.config/bottle/bottle.db`.

Error types and logging can stay small with custom error types rather than
`thiserror` or `anyhow`. They are not part of the public interface.

## Crate layout

```
src/lib.rs         library: open db, run verbs, format output
src/bin/bottle.rs  clap, env, process I/O, exit codes
tests/             integration tests, one file per command
```

The library's public entry is structured commands, not
stringly argv. CLI and MCP both call that. `src/bin/bottle.rs`
does not open sqlite or format TSV.

## Database

One sqlite file. bottle creates a `schemas` registry table,
a `links` table, and one data table per schema. See
[schema.md](schema.md).
Default path: Linux XDG data dir, macOS
`~/.config/bottle/bottle.db`. `--db` / `BOTTLE_DB` override
it. Create the parent directory if needed.

On open:

```
PRAGMA journal_mode=WAL;
PRAGMA busy_timeout=5000;
PRAGMA user_version=2;
```

`user_version` is 2: field catalog as rows, not a YAML blob.
A file with a higher version is refused. Version 0 and 1
files that still have `schemas.spec` are migrated on open.

WAL so readers do not block a writer. `busy_timeout` is
5000 milliseconds: a second writer waits up to that long
instead of failing immediately. Several bots, one file.

## Tests

`tests/` drives the library against a temp db. Cover every
implemented command in [commands.md](commands.md): happy
path, each
flag, each documented failure (usage, unknown schema, bad
field, not found, retired, link target missing, reserved
`--where`, `--link`/`--unlink` conflict), TSV headers and
cells (`true`/`false`, numbers as logged,
`links` sort), and time bounds including date-only vs
instant. MCP `log` `entries` is one transaction: all
succeed or none do.

The binary gets a thin smoke: clap maps flags into the
same structured commands, exit `0`/`1`/`2`, stdout vs
stderr. Do not duplicate the verb matrix there.

`cargo test` is the suite. No network. No machine-wide
db path.

## Time

jiff reads the host timezone. Storage is UTC `Z`. Output is
offset-local. See [time.md](time.md).

## MCP

`bottle mcp` is a stdio server using rmcp. Same verbs as the
CLI. See [mcp.md](mcp.md).

## Release binary

Target: `x86_64-unknown-linux-musl`.

```
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

The artifact is `target/x86_64-unknown-linux-musl/release/bottle`.
Copy that file. Do not ship `target/`, the crate, or a
toolchain next to it.

Other OS targets (macOS, Windows) are fine for development.
The Linux release is musl and static.
