# DEV.md — gitforge for contributors

## Architecture overview

```mermaid
flowchart LR
    A[AI Client] -- stdin/stdout<br/>JSON-RPC 2.0 --> B[gitforge]
    B --> C[main.rs]
    C --> D[cli.rs<br/>clap parsing]
    C --> E[log.rs<br/>logger]
    C --> F[mcp/router.rs<br/>Router]
    C --> G[mcp/transport.rs<br/>stdio loop]
    F --> H[tools/mod.rs<br/>7 tool handlers]
    F --> I[resources: git://HEAD<br/>git://status]
    H --> J[git/repo.rs<br/>RepoHandle actor]
    I --> J
    J --> K[git2::Repository<br/>(spawned thread)]
```

The server starts, parses CLI args, opens the Git repository on a
background thread (via `RepoHandle`), registers tools and resources on
the `Router`, then enters a line-delimited JSON-RPC read loop on stdin.
Each inbound request is dispatched by the Router to the appropriate tool
handler or resource fetcher, which communicates with the Git actor via
`mpsc` channels.

## Server API (MCP methods)

All methods use line-delimited JSON-RPC 2.0 over stdin/stdout.

### `initialize`

**Request:** standard MCP initialize with client capabilities.

**Response:**

```json
{
  "protocolVersion": "2025-11-25",
  "capabilities": { "tools": {}, "resources": {} },
  "serverInfo": { "name": "gitforge", "version": "0.1.0" },
  "tools": [ /* list of all registered tools */ ],
  "resources": [ /* list of all registered resources */ ]
}
```

The `initialize` response includes both `tools` and `resources` arrays —
this is optional per the MCP spec but simplifies client setup.

### `ping`

**Request:** `{"jsonrpc":"2.0","id":1,"method":"ping"}`

**Response:** `{"jsonrpc":"2.0","id":1,"result":{}}`

### `tools/list`

**Response:** returns all 7 registered tools with name, description, and
input schema.

### `tools/call`

**Request params:** `{"name":"<tool>","arguments":{...}}`

**Response:** `{"content":[{"type":"text","text":"<output>"}]}`

| Tool           | Arguments                                           | Output                                       |
| -------------- | --------------------------------------------------- | -------------------------------------------- |
| `ping`         | none                                                | `"pong"`                                     |
| `git_status`   | none                                                | `"nothing to commit..."` or per-file lines   |
| `git_log`      | `max_count` (int, default 10)                       | `"<hash>  <author>  <subject>"` per commit   |
| `git_branches` | none                                                | `"* main"` etc, `*` marks HEAD               |
| `git_diff`     | none                                                | unified diff HEAD→workdir, or `"no changes"` |
| `git_show`     | `revision` (str, default `"HEAD"`)                  | commit + author + date + message + diff      |
| `git_commit`   | `message`, `author_name`, `author_email` (required) | `"Created commit <hash>"`                    |

### `resources/list`

**Response:** returns 2 resources: `git://HEAD` and `git://status`.

### `resources/read`

**Request params:** `{"uri":"git://HEAD"}`

**Response:** `{"contents":[{"uri":"...","mimeType":"text/plain","text":"..."}]}`

| URI            | Text format                                                              |
| -------------- | ------------------------------------------------------------------------ |
| `git://HEAD`   | `commit <hash>\nAuthor: ...\nDate: ...\n\n<message>`                     |
| `git://status` | `"nothing to commit, working tree clean"` or `<STATUS>  <path>` per file |

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

Feature flags (Cargo features, similar to `-D` flags in C Makefiles):

| Feature                | Effect                                | Dep      |
| ---------------------- | ------------------------------------- | -------- |
| `show_time_stamp`      | Enable chrono-based timestamps in log | `chrono` |
| `show_source_location` | Enable `[file:line:func]` in log      | none     |

Features are additive — enable both for full log output. Default = none
(minimal log format). See `AGENTS.md` for the canonical test command.

## Repo layout

```
src/
├── main.rs            entrypoint
├── cli.rs             clap CLI argument parsing
├── error.rs           GitforgeError enum (6 variants)
├── log.rs             thread-safe logger (7 levels, feature-gated)
├── git/
│   ├── mod.rs         re-export
│   └── repo.rs        RepoHandle actor (7 command variants)
├── mcp/
│   ├── mod.rs         re-export Router
│   ├── types.rs       JSON-RPC 2.0 types (serde)
│   ├── router.rs      method dispatcher, tools HashMap, resources Vec
│   └── transport.rs   stdin/stdout line-delimited loop
├── tools/
│   └── mod.rs         7 tool registrations
tests/
└── integration.rs     14 integration tests
```

## Concurrency

- **RepoHandle actor** — `git2::Repository` is `!Send`, so it lives on a
  dedicated thread. Commands are sent via `mpsc::Sender<RepoCommand>`,
  responses via a one-shot `mpsc::Sender<Result>`. The actor thread
  processes commands sequentially (single queue).
- **Logger** — `std::sync::Mutex` guards the writer. Lock held for the
  entire format-and-write cycle to prevent interleaved output.
- **No async runtime.** All I/O (stdin, channels) is blocking. The stdio
  read loop is single-threaded after startup.
- **Rate limiting / queuing:** Not implemented. The `mpsc` channel has
  unbounded buffer; rapid-fire requests queue in memory. No request
  deduplication or timeout.

## Testing

14 integration tests in `tests/integration.rs`:

| Test                                          | What it verifies                          |
| --------------------------------------------- | ----------------------------------------- |
| `test_ping`                                   | ping → `{}`                               |
| `test_initialize_returns_tools_and_resources` | initialize includes tools[] + resources[] |
| `test_invalid_method`                         | unknown method → error code -32601        |
| `test_git_branches`                           | lists at least one branch                 |
| `test_git_status_clean`                       | clean repo → "nothing to commit"          |
| `test_git_status_dirty`                       | modified file → WT_MODIFIED               |
| `test_git_log`                                | lists commit subjects                     |
| `test_git_diff`                               | shows diff content                        |
| `test_git_show`                               | returns commit + author                   |
| `test_resources_list`                         | lists git://HEAD + git://status           |
| `test_resource_read_head`                     | reads HEAD commit info                    |
| `test_resource_read_status`                   | reads clean status                        |
| `test_resource_unknown_uri`                   | unknown URI → error                       |
| `test_notification_no_response`               | notification → no output                  |

Tests create a temporary Git repo with 2 commits via the `git` CLI, then
spawn the binary using `CARGO_BIN_EXE_gitforge`.

## Coding conventions

- **Formatting:** `.rustfmt.toml` enforces `hard_tabs = true`,
  `tab_spaces = 4` — run `cargo fmt` before committing.
- **Logging:** Use the `log_*!` macros (`log_info!`, `log_error!`, etc.)
  instead of `println!`.
- **Error handling:** Use `GitforgeError` enum with `thiserror` derive.
  Prefer channel-based error propagation from the actor; map to
  `GitforgeError` variants.
- **No `unwrap()` in production paths** — target code uses `?` and
  `map_err`. `unwrap()` is acceptable in tests and in `Mutex::lock()`
  paths (poison is a fatal error).

## External dependencies

| Crate        | Version        | Purpose                                   |
| ------------ | -------------- | ----------------------------------------- |
| `clap`       | 4.6.4          | CLI argument parsing                      |
| `git2`       | 0.21.0         | Git repository access (libgit2 bindings)  |
| `serde`      | 1.0.229        | JSON serialization for MCP                |
| `serde_json` | 1.0.151        | JSON value types                          |
| `thiserror`  | 2.0.19         | Error enum derive                         |
| `anyhow`     | 1.0.104        | Error context (minimal use)               |
| `chrono`     | 0.4 (optional) | Local-time timestamps (`show_time_stamp`) |
| `tempfile`   | 3 (dev)        | Temporary repos in integration tests      |

See [DEV_IN_DEPTH.md](DEV_IN_DEPTH.md) for implementation details.
