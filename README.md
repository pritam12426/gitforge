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

- **7 MCP tools:** ping, git_status, git_log, git_branches, git_diff,
  git_show, git_commit
- **2 MCP resources:** `git://HEAD` and `git://status`
- **Actor-based Git access:** `git2::Repository` runs on its own thread
  (required because `git2::Repository` is `!Send`)
- **Line-delimited JSON-RPC 2.0** over stdin/stdout
- **Thread-safe logger** with 7 levels, ANSI color, optional timestamps
  and source-location (feature-gated)
- **14 integration tests** covering all tools, resources, and error paths

## Install

```sh
cargo install --path .
```

Requires Rust 1.96+. Builds on any platform Rust supports (tested on macOS).

## Quickstart

```sh
# Start the server for the current repo (listens on stdin/stdout)
gitforge .

# In another terminal, test with a JSON-RPC request:
echo '{"jsonrpc":"2.0","id":1,"method":"ping"}' | gitforge .
```

In practice, the AI client (Claude Code, etc.) launches gitforge as a
subprocess and communicates over its stdin/stdout automatically.

## Basic usage

```sh
gitforge /path/to/repo                    # default log level: info
gitforge . -l debug                       # verbose logging
gitforge . --log-file /tmp/gitforge.log   # log to file
```

### CLI options

| Argument          | Description                                                          |
| ----------------- | -------------------------------------------------------------------- |
| `[REPO_PATH]`     | Repo path (positional, defaults to `.`)                              |
| `-r, --repo`      | Explicit repo path (overrides positional)                            |
| `--log-file`      | Log file path (default: stderr)                                      |
| `-l, --log-level` | Min level: `off`, `fatal`, `error`, `warn`, `info`, `debug`, `trace` |

## Build

```sh
cargo build              # debug build
cargo build --release    # release build (no custom optimisations yet)
cargo check              # fast type-check only
cargo fmt                # formats with hard_tabs, tab_spaces=4
```

Rust edition 2024. `Cargo.lock` is committed for binary reproducibility.

## Platform support

Targets any platform Rust supports — built and tested on macOS.

## License

MIT — see [LICENSE](LICENSE)

## Contributing

See [DEV.md](DEV.md) for contributor guidance and
[DEV_IN_DEPTH.md](DEV_IN_DEPTH.md) for the full source walkthrough.
