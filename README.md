# gitforge

**gitforge** is a Git MCP (Model Context Protocol) server written in Rust.
It bridges AI assistants like Claude to Git repositories by exposing version
control operations as MCP tools.

> **Status:** early scaffold — CLI parsing and logging exist; the MCP protocol
> layer and Git integration are not yet implemented. See [ROUGH_IDEA.md] for
> the architectural plan.

## Current capabilities

- CLI argument parsing via `clap` (derive API)
- Thread-safe logger (7 levels, ANSI color, timestamps, source location)
- Zero dependencies beyond `clap`

## Build

```sh
cargo build              # debug build
cargo build --release    # release build
cargo check              # fast type-check only
```

Rust edition 2024. `Cargo.lock` is committed for binary reproducibility.

## Usage

```sh
# Show available options
cargo run -- --help

# Run with defaults (repo=., log-level=info, stderr)
cargo run .

# Specify repo path and log level
cargo run /path/to/repo -l debug

# Log to a file, suppress timestamps and color
cargo run --repo /path/to/repo --log-file /tmp/gitforge.log --no-color --no-timestamp
```

### CLI options

| Argument          | Description                                                          |
| ----------------- | -------------------------------------------------------------------- |
| `[REPO_PATH]`     | Repo path (positional, defaults to `.`)                              |
| `-r, --repo`      | Explicit repo path (overrides positional)                            |
| `--log-file`      | Log file path (default: stderr)                                      |
| `-l, --log-level` | Min level: `off`, `fatal`, `error`, `warn`, `info`, `debug`, `trace` |
| `--no-color`      | Disable ANSI color                                                   |
| `--no-timestamp`  | Suppress timestamps                                                  |
| `--no-source`     | Suppress `[file:line]` in output                                     |

## Platform support

Targets any platform Rust supports — built and tested on macOS.

## License

MIT — see [LICENSE](LICENSE)

## Contributing

This is a very early-stage project. See [DEV.md](DEV.md) for contributor
guidance and [DEV_IN_DEPTH.md](DEV_IN_DEPTH.md) for the full source
walkthrough.
