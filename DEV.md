# DEV.md — gitforge for contributors

## Architecture overview

```
┌─────────────────────┐
│      main.rs        │  entrypoint: parse CLI → init logger → (future: MCP loop)
├─────────────────────┤
│      cli.rs         │  clap-derive Args struct
├─────────────────────┤
│      log.rs         │  thread-safe logger (7 levels, color, timestamps, source)
└─────────────────────┘
```

The project is a single Rust **binary crate**. At HEAD, it parses CLI arguments,
initializes the logger, logs a starting message, and exits. The MCP transport
layer and Git tool modules are not yet implemented. See [ROUGH_IDEA.md] for
the intended architecture.

## CLI reference

```
gitforge [OPTIONS] [REPO_PATH]
```

### Arguments

| Position       | Name        | Type              | Default | Description          |
| -------------- | ----------- | ----------------- | ------- | -------------------- |
| positional     | `REPO_PATH` | `PathBuf`         | `.`     | Git repository path  |
| `-r`, `--repo` | `repo`      | `Option<PathBuf>` | —       | Overrides positional |

### Options

| Flag                | Type              | Default | Description                                                                                            |
| ------------------- | ----------------- | ------- | ------------------------------------------------------------------------------------------------------ |
| `--log-file`        | `Option<PathBuf>` | stderr  | Log output file path                                                                                   |
| `-l`, `--log-level` | `Level`           | `info`  | Minimum severity. Values (case-insensitive): `off`, `fatal`, `error`, `warn`, `info`, `debug`, `trace` |
| `--no-color`        | `bool`            | `false` | Disable ANSI color (auto-detected when writing to a TTY)                                               |
| `--no-timestamp`    | `bool`            | `false` | Suppress `[HH:MM:SS.ffffff]` prefix                                                                    |
| `--no-source`       | `bool`            | `false` | Suppress `[file:line]` suffix                                                                          |
| `-h`, `--help`      | —                 | —       | Print help                                                                                             |

### Runtime behaviour

- If both `REPO_PATH` (positional) and `--repo` are given, `--repo` wins.
- Log-level filtering: a message at `level > current_level` is suppressed
  (higher numeric value = lower priority). So at level `warn`, messages at
  `info`, `debug`, `trace` are dropped.

## Module responsibilities

### `src/main.rs`

- Declares `mod cli` and `mod log`.
- Calls `Cli::parse()` (from clap's `Parser` derive).
- Calls `log::init()` with parsed config.
- Emits a single `log_info!` message on startup.

### `src/cli.rs`

- `Args` struct with clap `#[derive(Parser)]`.
- `Args::effective_repo_path()` — returns `--repo` value or falls back to
  positional `repo_path`.

### `src/log.rs`

Thread-safe logger with no dependencies beyond `clap::ValueEnum` (used for the
log-level CLI argument).

**Exported items:**

| Item                             | Kind               | Description                                                  |
| -------------------------------- | ------------------ | ------------------------------------------------------------ |
| `Level`                          | enum + `ValueEnum` | `Off                                                         |
| `init()`                         | function           | Configure output path, level, timestamp/source/color toggles |
| `set_level()`                    | function           | Runtime level change                                         |
| `level()`                        | function           | Current level                                                |
| `use_color()`                    | function           | Whether color is enabled                                     |
| `set_show_timestamp()`           | function           | Toggle timestamp output                                      |
| `set_show_source()`              | function           | Toggle source-location output                                |
| `set_color()`                    | function           | Toggle ANSI color                                            |
| `__log_impl()`                   | function           | Core writer (called by macros)                               |
| `log!()` / `log_!()`             | macro              | General-purpose (with/without trailing newline)              |
| `log_fatal!()` .. `log_trace!()` | macro              | Level-specific shorthands                                    |

**Log output format (all features enabled, color):**

```
[HH:MM:SS.ffffff] [LEVEL] [file:line] message
```

Color dims the timestamp, applies level-specific color to the label, dims the
source location, and prepends a level-specific emoji.

## Concurrency

- Logger uses `std::sync::Mutex` — safe across threads, single-writer.
- There is no async runtime, no MCP transport, no request handling yet.
  Concurrency semantics for those layers are TBD.

## Testing

No tests exist at HEAD. The test framework is not yet chosen.

## Build & tooling

```sh
cargo build              # debug
cargo build --release    # release (no custom profile settings yet)
cargo check              # fast type-check
cargo fmt                # formats with hard_tabs, tab_spaces=4
```

## Coding conventions

- **Formatting:** `.rustfmt.toml` enforces `hard_tabs = true`,
  `tab_spaces = 4` — run `cargo fmt` before committing.
- **Logging:** Use the `log_*!` macros from `log.rs` instead of `println!`.
- **Error handling:** No error-handling conventions established yet (the binary
  is a scaffold).

## External dependencies

| Crate  | Version    | Purpose                                  |
| ------ | ---------- | ---------------------------------------- |
| `clap` | 4 (derive) | CLI argument parsing, generates `--help` |
