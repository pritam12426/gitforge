# DEV.md — gitforge for contributors

## Architecture overview

```mermaid
flowchart LR
    A[AI Client] -- "stdin/stdout JSON-RPC 2.0" --> B[lib.rs<br/>run(Cli)]
    B --> C[cli.rs<br/>clap parse]
    B --> D[logging::<br/>logger + macros]
    B --> E[transport::stdio::<br/>line-delimited loop]
    E --> F[mcp/router.rs]
    F --> G[tools/*.rs<br/>13 tool handlers]
    F --> H[mcp/resources.rs<br/>git://HEAD, git://status]
    G --> I[git/commands.rs<br/>RepoCommand enum]
    H --> I
    I --> J[git/actor.rs<br/>dispatch loop]
    J --> K[git/ops/*.rs<br/>git2 operations]
    E -.->|roots/list<br/>after initialize| L[MCP Roots<br/>discovery]
    L --> F
```

The server parses CLI args, opens the Git repository on a background
thread via a `RepoHandle` (actor pattern), registers 13 tools and 2
resources on the `Router`, then starts the stdio read loop. Each request
dispatches through the Router to the appropriate handler, which sends a
command to the Git actor via `mpsc` channels and awaits the response.

After the `initialize` handshake, if the client declares `roots` support,
the server sends a synchronous `roots/list` request and stores the
returned paths on the router before entering the main loop.

## Server API (MCP methods)

All methods use line-delimited JSON-RPC 2.0 over stdin/stdout.

### `initialize`

**Request:** standard MCP initialize with client capabilities.

**Response:**

```json
{
  "protocolVersion": "2025-11-25",
  "capabilities": { "tools": { "listChanged": false }, "resources": {} },
  "serverInfo": { "name": "gitforge", "version": "0.2.0" },
  "tools": [ /* list of all registered tools */ ],
  "resources": [ /* list of all registered resources */ ]
}
```

The response includes both `tools` and `resources` arrays — optional per
the MCP spec, but simplifies client setup.

### `ping`

**Request:** `{"jsonrpc":"2.0","id":1,"method":"ping"}`

**Response:** `{"jsonrpc":"2.0","id":1,"result":{}}`

### `tools/list`

**Response:** returns all 13 registered tools with name, description,
input schema, and `ToolAnnotations` (read_only/destructive/mutable hints).

### `tools/call`

**Request params:** `{"name":"<tool>","arguments":{...}}`

**Response:** `{"content":[{"type":"text","text":"<output>"}]}`

| Tool                  | Arguments                                          | Output                                            |
| --------------------- | -------------------------------------------------- | ------------------------------------------------- |
| `ping`                | none                                               | `"pong"`                                          |
| `git_status`          | none                                               | `"nothing to commit..."` or per-file lines        |
| `git_log`             | `offset` (int, default 0), `limit` (int, default 10) | `"<hash> <author> <subject>"` per commit        |
| `git_branches`        | `branch_type` (str, default `"local"`), `contains` (str, optional), `not_contains` (str, optional) | `"* main"` etc, `*` marks HEAD |
| `git_diff`            | `target` (str, required) — branch/commit/ref to compare HEAD against | unified diff, or `"no differences"` |
| `git_diff_unstaged`   | none                                               | unified diff workdir→index, or `"no unstaged changes"` |
| `git_diff_staged`     | none                                               | unified diff index→HEAD, or `"no staged changes"` |
| `git_show`            | `revision` (str, default `"HEAD"`)                 | commit + author + date + message + diff           |
| `git_commit`          | `message`, `author_name`, `author_email` (required)| `"Created commit <hash>"`                         |
| `git_add`             | `files` (array of strings, required)               | `"staged"`                                        |
| `git_branch_create`   | `name` (str, required), `revision` (str, optional, default `"HEAD"`) | `"Created branch '<name>'"`         |
| `git_checkout`        | `branch` (str, required)                           | `"switched branch"`                               |
| `git_merge`           | `branch` (str, required)                           | merge result message from git2                    |

Flag injection defense: `git_show`, `git_log`, `git_branch_create`,
`git_checkout`, `git_branches`, and `git_diff` reject arguments starting
with `-` before passing them to the git layer.

### `resources/list`

**Response:** returns 2 resources: `git://HEAD` and `git://status`.

### `resources/read`

**Request params:** `{"uri":"git://HEAD"}`

**Response:** `{"contents":[{"uri":"...","mimeType":"text/plain","text":"..."}]}`

| URI            | Text format                                                              |
| -------------- | ------------------------------------------------------------------------ |
| `git://HEAD`   | `commit <hash>\nAuthor: ...\nDate: ...\n\n<message>`                     |
| `git://status` | `"nothing to commit, working tree clean"` or `<STATUS> <path>` per file  |

### Notifications

`notifications/initialized` and `notifications/cancelled` are accepted
but produce no response. Unknown methods return error code `-32601`.

## Build system

```sh
cargo build              # debug
cargo build --release    # release (no custom profile settings)
cargo check              # fast type-check
cargo fmt                # hard_tabs, tab_spaces=4
cargo test --features "show_time_stamp,show_source_location"
```

Feature flags:

| Feature                | Effect                                | Dep      |
| ---------------------- | ------------------------------------- | -------- |
| `show_time_stamp`      | Enable chrono-based timestamps in log | `chrono` |
| `show_source_location` | Enable `[file:line:func]` in log      | none     |

Default = none (minimal log format). See `AGENTS.md` for the canonical
test command.

## Repo layout

```
src/
├── lib.rs              library entrypoint
├── main.rs             thin binary wrapper (~8 lines)
├── cli.rs              clap CLI argument parsing
├── error.rs            GitforgeError enum (9 variants)
├── git/
│   ├── mod.rs          re-export
│   ├── actor.rs        actor dispatch loop
│   ├── commands.rs     RepoCommand enum (13 variants)
│   ├── handle.rs       RepoHandle (sender side)
│   └── ops/
│       ├── mod.rs      re-export
│       ├── status.rs
│       ├── log.rs
│       ├── branches.rs
│       ├── diff_unstaged.rs
│       ├── diff_staged.rs
│       ├── diff_target.rs
│       ├── show.rs
│       ├── commit.rs
│       ├── stage.rs
│       ├── branch_create.rs
│       ├── checkout.rs
│       └── merge.rs
├── logging/
│   ├── mod.rs          logger state + public API
│   └── macros.rs       log_*! macros
├── mcp/
│   ├── mod.rs          re-export
│   ├── types.rs        JSON-RPC 2.0 types (serde)
│   ├── router.rs       method dispatcher, tools HashMap
│   └── resources.rs    Resource struct + built-in resources
├── tools/
│   ├── mod.rs          common helpers + register_all
│   ├── ping.rs
│   ├── git_status.rs
│   ├── git_log.rs
│   ├── git_branches.rs
│   ├── git_diff.rs
│   ├── git_diff_unstaged.rs
│   ├── git_diff_staged.rs
│   ├── git_show.rs
│   ├── git_commit.rs
│   ├── git_add.rs
│   ├── git_branch_create.rs
│   ├── git_checkout.rs
│   └── git_merge.rs
├── transport/
│   ├── mod.rs          re-export
│   └── stdio.rs        stdin/stdout line-delimited loop
tests/
└── integration.rs      18 integration tests
```

## Concurrency

- **Git actor** — `git2::Repository` is `!Send`, so it lives on a
  dedicated thread. Commands are sent via `mpsc::Sender<RepoCommand>`,
  responses via a one-shot channel. The actor processes commands
  sequentially.
- **Request timeout** — `recv_response` blocks for at most 30 seconds
  (`REQUEST_TIMEOUT`), returning `GitforgeError::Actor` on timeout.
- **Logger** — `std::sync::Mutex` guards the writer. Lock held for the
  entire format-and-write cycle.
- **No rate limiting / queuing** — The `mpsc` channel is unbounded;
  rapid-fire requests queue in memory. No request deduplication.

## Testing

18 integration tests in `tests/integration.rs`:

| Test                                                      | What it verifies                          |
| --------------------------------------------------------- | ----------------------------------------- |
| `test_ping`                                               | ping → `{}`                               |
| `test_initialize_returns_tools_and_resources`             | initialize includes tools[] + resources[] |
| `test_unknown_method`                                     | unknown method → error code -32601        |
| `test_parse_error`                                        | malformed JSON → error -32700             |
| `test_notification_no_response`                           | notification → no output                  |
| `test_git_branches_filtered`                              | filters branches by type, contains/not_contains |
| `test_git_status_clean`                                   | clean repo → "nothing to commit"          |
| `test_git_status_dirty`                                   | modified file → WT_MODIFIED               |
| `test_git_log_default_and_limit`                          | lists commit subjects, respects limit     |
| `test_git_diff_target`                                    | diff HEAD vs target branch                |
| `test_git_diff_unstaged_and_staged`                       | unstaged and staged diff output           |
| `test_git_show_head`                                      | returns commit + author                   |
| `test_git_show_unknown_revision`                          | unknown revision → error                  |
| `test_git_add_and_commit`                                 | stage + commit workflow                   |
| `test_git_branch_create_and_checkout`                     | create branch + switch to it              |
| `test_git_merge_fast_forward_and_already_up_to_date`     | merge two branches twice                  |
| `test_resources_list_and_read`                            | read git://HEAD and git://status          |
| `test_full_mcp_session`                                   | end-to-end initialize-ping-tools-resources|

Tests create a temporary Git repo with 2 commits via the `git2` API,
then spawn the binary using `CARGO_BIN_EXE_gitforge`.

## Coding conventions

- **Formatting:** `.rustfmt.toml` enforces `hard_tabs = true`,
  `tab_spaces = 4` — run `cargo fmt` before committing.
- **Logging:** Use the `log_*!` macros (`log_info!`, `log_error!`, etc.)
  instead of `println!`. The logger supports `--log-format json` for
  machine-parseable output and respects `GITFORGE_LOG_LEVEL` env var.
- **Error handling:** Use `GitforgeError` enum with `thiserror` derive.
  Prefer channel-based error propagation from the actor; map to
  `GitforgeError` variants. Use `GitforgeError::rpc_code()` to map
  errors to JSON-RPC error codes.
- **No `unwrap()` in production paths** — target code uses `?` and
  `map_err`. `unwrap()` is acceptable in tests and in `Mutex::lock()`
  paths (poison is a fatal error). `String::from_utf8(...).unwrap()` in
  diff ops is safe because the byte buffer is built from validated
  `&str` content.
- **Flag injection defense** — tool handlers that accept user-supplied
  strings call `reject_flag(value, field)` to reject values starting
  with `-`.

## External dependencies

| Crate               | Version    | Purpose                                    |
| ------------------- | ---------- | ------------------------------------------ |
| `clap`              | 4.6.4      | CLI argument parsing                          |
| `git2`              | 0.21.0     | Git repository access (libgit2 bindings)   |
| `serde`             | 1.0.229    | JSON serialization                         |
| `serde_json`        | 1.0.151    | JSON value types                           |
| `thiserror`         | 2.0.19     | Error enum derive                          |
| `chrono`            | 0.4 (opt)  | Local-time timestamps (`show_time_stamp`)  |
| `tempfile`          | 3 (dev)    | Temporary repos in integration tests       |

See [DEV_IN_DEPTH.md](DEV_IN_DEPTH.md) for implementation details.
