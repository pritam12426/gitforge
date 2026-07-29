# DEV.md — gitforge for contributors

## Architecture overview

```mermaid
flowchart LR
    A[AI Client] -- "stdin/stdout JSON-RPC 2.0" --> B[gitforge stdio]
    A -- "POST /rpc JSON-RPC 2.0" --> C[gitforge HTTP]
    B --> D[lib.rs<br/>run(Args)]
    C --> D
    D --> E[cli.rs<br/>clap parse]
    D --> F[logging::<br/>logger + macros]
    D --> G[transport::stdio::<br/>line-delimited loop]
    D --> G2[transport::http::<br/>axum server]
    G --> H[mcp/router.rs]
    G2 --> H
    H --> I[tools/*.rs<br/>11 tool handlers]
    H --> J[mcp/resources.rs<br/>git://HEAD, git://status]
    I --> K[git/commands.rs<br/>RepoCommand enum]
    J --> K
    K --> L[git/actor.rs<br/>dispatch loop]
    L --> M[git/ops/*.rs<br/>git2 operations]
```

The server parses CLI args, opens the Git repository on a background
thread via a `RepoHandle` (actor pattern), registers 11 tools and 2
resources on the `Router`, then starts either the stdio loop or HTTP
server depending on the subcommand. Each request dispatches through
the Router to the appropriate handler, which sends a command to the
Git actor via `mpsc` channels and awaits the response.

## Server API (MCP methods)

All methods use JSON-RPC 2.0. In stdio mode it's line-delimited over
stdin/stdout; in HTTP mode it's `POST /rpc`.

### `initialize`

**Request:** standard MCP initialize with client capabilities.

**Response:**

```json
{
  "protocolVersion": "2025-11-25",
  "capabilities": { "tools": {}, "resources": {} },
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

**Response:** returns all 11 registered tools with name, description,
and input schema.

### `tools/call`

**Request params:** `{"name":"<tool>","arguments":{...}}`

**Response:** `{"content":[{"type":"text","text":"<output>"}]}`

| Tool                | Arguments                                          | Output                                            |
| ------------------- | -------------------------------------------------- | ------------------------------------------------- |
| `ping`              | none                                               | `"pong"`                                          |
| `git_status`        | none                                               | `"nothing to commit..."` or per-file lines        |
| `git_log`           | `offset` (int, default 0), `limit` (int, default 10) | `"<hash> <author> <subject>"` per commit        |
| `git_branches`      | none                                               | `"* main"` etc, `*` marks HEAD                    |
| `git_diff`          | none                                               | unified diff HEAD→workdir, or `"no changes"`      |
| `git_show`          | `revision` (str, default `"HEAD"`)                 | commit + author + date + message + diff           |
| `git_commit`        | `message`, `author_name`, `author_email` (required)| `"Created commit <hash>"`                         |
| `git_add`           | `files` (array of strings, required)               | `"staged"`                                        |
| `git_branch_create` | `name` (str, required), `revision` (str, optional, default `"HEAD"`) | `"Created branch '<name>'"`         |
| `git_checkout`      | `branch` (str, required)                          | `"switched branch"`                               |
| `git_merge`         | `branch` (str, required)                           | merge result message from git2                    |

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

### HTTP transport

When running as `gitforge server`, the server exposes two endpoints:

| Endpoint    | Method | Purpose                         |
| ----------- | ------ | ------------------------------- |
| `POST /rpc` | POST   | JSON-RPC 2.0 request            |
| `GET /health` | GET  | Health check — returns `200 OK` with `"ok"` |

The HTTP server reads the request body, deserializes a JSON-RPC request,
dispatches it via the Router (on a `spawn_blocking` thread), and writes
the response. Each request gets a unique correlation ID for logging.

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
│   ├── commands.rs     RepoCommand enum (11 variants)
│   ├── handle.rs       RepoHandle (sender side)
│   └── ops/
│       ├── mod.rs      re-export
│       ├── status.rs
│       ├── log.rs
│       ├── branches.rs
│       ├── diff.rs
│       ├── show.rs
│       ├── commit.rs
│       ├── add.rs
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
│   ├── git_show.rs
│   ├── git_commit.rs
│   ├── git_add.rs
│   ├── git_branch_create.rs
│   ├── git_checkout.rs
│   └── git_merge.rs
├── transport/
│   ├── mod.rs          re-export
│   ├── stdio.rs        stdin/stdout line-delimited loop
│   └── http.rs         axum server (POST /rpc, GET /health)
tests/
└── integration.rs      18 integration tests
```

## Concurrency

- **Git actor** — `git2::Repository` is `!Send`, so it lives on a
  dedicated thread. Commands are sent via `mpsc::Sender<RepoCommand>`,
  responses via a one-shot channel. The actor processes commands
  sequentially.
- **Logger** — `std::sync::Mutex` guards the writer. Lock held for the
  entire format-and-write cycle.
- **HTTP mode** — tokio multi-thread runtime. Each request is handled
  by an axum route that calls `spawn_blocking` to invoke the synchronous
  Router (which communicates with the Git actor). This prevents the
  synchronous `mpsc::recv()` from blocking the async runtime.
- **No rate limiting / queuing** — The `mpsc` channel is unbounded;
  rapid-fire requests queue in memory. No request deduplication or
  timeout.

## Testing

18 integration tests in `tests/integration.rs`:

| Test                                          | What it verifies                          |
| --------------------------------------------- | ----------------------------------------- |
| `test_ping`                                   | ping → `{}`                               |
| `test_initialize_returns_tools_and_resources` | initialize includes tools[] + resources[] |
| `test_unknown_method`                         | unknown method → error code -32601        |
| `test_parse_error`                            | malformed JSON → error -32700             |
| `test_notification_no_response`               | notification → no output                  |
| `test_git_branches`                           | lists at least one branch                 |
| `test_git_status_clean`                       | clean repo → "nothing to commit"          |
| `test_git_status_dirty`                       | modified file → WT_MODIFIED               |
| `test_git_log_default_and_limit`              | lists commit subjects, respects limit     |
| `test_git_diff`                               | shows diff content                        |
| `test_git_show_head`                          | returns commit + author                   |
| `test_git_show_unknown_revision`              | unknown revision → error                  |
| `test_git_add_and_commit`                     | stage + commit workflow                   |
| `test_git_branch_create_and_checkout`         | create branch + switch to it              |
| `test_git_merge_fast_forward_and_already_up_to_date` | merge two branches twice       |
| `test_resources_list_and_read`                | read git://HEAD and git://status          |
| `test_http_health`                            | GET /health → 200                         |
| `test_http_ping_rpc`                          | POST /rpc ping → valid response           |
| `test_http_parse_error`                       | POST /rpc malformed → error -32700        |

Tests create a temporary Git repo with 2 commits via the `git2` API,
then either spawn the binary (stdio tests) or use
`tower::ServiceExt::oneshot` (HTTP tests).

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
  paths (poison is a fatal error).

## External dependencies

| Crate               | Version    | Purpose                                    |
| ------------------- | ---------- | ------------------------------------------ |
| `clap`              | 4.6.4      | CLI argument parsing (features: derive, env)|
| `git2`              | 0.21.0     | Git repository access (libgit2 bindings)   |
| `serde`             | 1.0.229    | JSON serialization                         |
| `serde_json`        | 1.0.151    | JSON value types                           |
| `thiserror`         | 2.0.19     | Error enum derive                          |
| `anyhow`            | 1.0.104    | Error context (minimal use)                |
| `chrono`            | 0.4 (opt)  | Local-time timestamps (`show_time_stamp`)  |
| `axum`              | 0.7        | HTTP server                                |
| `tokio`             | 1          | Async runtime for HTTP mode                |
| `tower`             | 0.4 (dev)  | Service test harness for HTTP tests        |
| `http-body-util`    | 0.1 (dev)  | Body utilities for HTTP tests              |
| `tempfile`          | 3 (dev)    | Temporary repos in integration tests       |

See [DEV_IN_DEPTH.md](DEV_IN_DEPTH.md) for implementation details.
