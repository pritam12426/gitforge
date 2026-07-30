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

- **13 MCP tools:** ping, git_status, git_log, git_branches, git_diff,
  git_diff_unstaged, git_diff_staged, git_show, git_commit, git_add,
  git_branch_create, git_checkout, git_merge
- **2 MCP resources:** `git://HEAD` and `git://status`
- **Flag-injection defense:** tool arguments starting with `-` are rejected
  before reaching the git layer
- **Tool annotations:** read-only/destructive/mutable hints for client UI
- **MCP Roots discovery:** detects repositories managed by the client
- **Actor-based Git access:** `git2::Repository` runs on its own thread
  (required because `git2::Repository` is `!Send`)
- **Line-delimited JSON-RPC 2.0** over stdin/stdout
- **Thread-safe logger** with 7 levels, ANSI color, optional timestamps
  and source-location (feature-gated), JSON output format, and env-var
  level override
- **18 integration tests** covering all tools, resources, and error paths

## Install

```sh
cargo install --path .
```

Builds on any platform Rust supports (tested on macOS).

## Quickstart

```sh
# Start the server for the current repo
gitforge

# Test with a JSON-RPC request:
echo '{"jsonrpc":"2.0","id":1,"method":"ping"}' | gitforge --repo .
```

In practice, the AI client (Claude Code, etc.) launches gitforge as a
subprocess and communicates over its stdin/stdout automatically.

## Basic usage

```sh
gitforge                              # default log level: warn
gitforge --repo /path/to/repo -l debug # verbose logging
gitforge --log-file /tmp/gitforge.log  # log to file
gitforge --log-format json             # machine-parseable logs
gitforge --allowed-repo /path/to/repo  # restrict operations to this tree
```

### CLI options

| Argument                | Description                                                          |
| ----------------------- | -------------------------------------------------------------------- |
| `--repo [PATH]`         | Repo path (default `.`, or `GITFORGE_REPO` env var)                  |
| `--allowed-repo [PATH]` | Restrict operations to this tree (defaults to `--repo`)              |
| `--log-file [PATH]`     | Log file path (default: stderr)                                      |
| `-l, --log-level`       | Min level: `off`, `fatal`, `error`, `warn`, `info`, `debug`, `trace` |
| `--log-format`          | Output format: `pretty` (default) or `json`                          |

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
