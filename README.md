# gitforge

**gitforge** is a Git MCP (Model Context Protocol) server written in Rust.
It bridges AI assistants to Git repositories by exposing version control
operations as MCP tools and resources.

## Why

AI assistants need controlled access to Git repositories — reading status,
history, diffs, and making commits. gitforge runs as a local stdio server
that the AI client launches as a subprocess, giving it narrow, audit-able
Git access without exposing the full shell.

## Features

- **11 MCP tools:** ping, git_status, git_log, git_branches, git_diff,
  git_show, git_commit, git_add, git_branch_create, git_checkout, git_merge
- **2 MCP resources:** `git://HEAD` and `git://status`
- **Two transport modes:** stdio (default) or HTTP (`gitforge server`)
- **Actor-based Git access:** `git2::Repository` runs on its own thread
  (required because `git2::Repository` is `!Send`)
- **Line-delimited JSON-RPC 2.0** over stdin/stdout, or HTTP JSON-RPC POST
- **Thread-safe logger** with 7 levels, ANSI color, optional timestamps
  and source-location (feature-gated), JSON output format, and env-var
  level override
- **18 integration tests** covering all tools, resources, HTTP, and error
  paths

## Install

```sh
cargo install --path .
```

Builds on any platform Rust supports (tested on macOS).

## Quickstart

```sh
# Start the server for the current repo in stdio mode
gitforge .

# Test with a JSON-RPC request:
echo '{"jsonrpc":"2.0","id":1,"method":"ping"}' | gitforge .
```

In practice, the AI client (Claude Code, etc.) launches gitforge as a
subprocess and communicates over its stdin/stdout automatically.

## Basic usage

```sh
gitforge --repo /path/to/repo           # default log level: info
gitforge --repo /path/to/repo -l debug   # verbose logging
gitforge --repo . --log-file /tmp/gitforge.log   # log to file
gitforge --repo . --log-format json      # machine-parseable logs
gitforge server --host 0.0.0.0 -P 8787   # HTTP mode
```

### CLI options

| Argument             | Description                                                          |
| -------------------- | -------------------------------------------------------------------- |
| `--repo [PATH]`      | Repo path (default `.`)                                              |
| `--log-file`         | Log file path (default: stderr)                                      |
| `-l, --log-level`    | Min level: `off`, `fatal`, `error`, `warn`, `info`, `debug`, `trace` |
| `--log-format`       | Output format: `pretty` (default) or `json`                          |
| `server`             | Subcommand to serve MCP over HTTP                                    |
| `server --host`      | Bind address for HTTP mode (default `127.0.0.1`)                     |
| `server -P, --port`  | Port for HTTP mode (default `8787`)                                  |

Log level can also be set via the `GITFORGE_LOG_LEVEL` environment
variable, which takes precedence over the CLI flag.

## Build

```sh
cargo build              # debug build
cargo build --release    # release build
cargo check              # fast type-check only
cargo fmt                # formats with hard_tabs, tab_spaces=4
cargo test --features "show_time_stamp,show_source_location"
```

Rust edition 2024. `Cargo.lock` is committed for binary reproducibility.

## License

MIT — see [LICENSE](LICENSE)

## Contributing

See [DEV.md](DEV.md) for contributor guidance and
[DEV_IN_DEPTH.md](DEV_IN_DEPTH.md) for the full source walkthrough.
