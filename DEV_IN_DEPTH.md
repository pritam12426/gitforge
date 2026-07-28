# DEV_IN_DEPTH.md — gitforge internal architecture

> Every statement below is traceable to the current HEAD commit. For the
> aspirational architecture (MCP transport, git2, tokio, tool modules) see
> [ROUGH_IDEA.md].

---

## 1. Project overview

`gitforge` is a single Rust binary crate (edition 2024) at version 0.1.0.
It is intended to become a Git MCP server. At HEAD it is an executable scaffold
consisting of:

- **CLI argument parser** (`src/cli.rs`) — clap derive
- **Logger** (`src/log.rs`) — thread-safe, zero-dep (except `clap::ValueEnum`)
- **Entrypoint** (`src/main.rs`) — parse args → init logger → exit

No MCP protocol handling, no Git operations, no async runtime exist yet.

---

## 2. Source tree

```
gitforge/
├── Cargo.toml          # single binary, clap dependency
├── Cargo.lock          # committed for binary reproducibility
├── LICENSE             # MIT
├── README.md           # user-facing docs
├── DEV.md              # contributor docs
├── DEV_IN_DEPTH.md     # this file
├── AGENTS.md           # AI-agent instruction file
├── ROUGH_IDEA.md       # forward-looking design document
├── REFERENCES.md       # developer reference links (empty template)
├── TODO.txt            # personal todo list (unrelated)
├── temp_todo.md        # scratchpad
├── .rustfmt.toml       # hard_tabs = true, tab_spaces = 4
├── .gitignore          # standard Rust + Cargo.lock committed
├── .gitattributes      # hidden, default
└── src/
    ├── main.rs         # entrypoint, 19 lines
    ├── cli.rs          # Args struct, 41 lines
    └── log.rs          # Logger, 246 lines
```

---

## 3. Execution flow: startup → shutdown

```
main()
  │
  ├─ cli::Args::parse()          ← clap parses argv; panics on invalid input
  │
  ├─ log::init(                  ← configures global logger
  │     args.log_file,           │  None → stderr
  │     args.log_level,          │  clap ValueEnum → Level
  │     !args.no_timestamp,      │
  │     !args.no_source,         │
  │     args.no_color,           │
  │   )
  │
  ├─ log_info!("gitforge starting — repo: {}", repo)
  │                              ← writes formatted line to configured output
  │
  └─ main() returns              ← program exits
```

### Initialisation details

`log::init()` is the only nontrivial initialisation path:

1. **If `path` is `Some(p)`:** attempt `OpenOptions::new().append(true).create(true).open(p)`.
   - On success: writer = file, color disabled (files are never TTY).
   - On failure: write a warning to stderr, fall back to stderr, auto-detect color
     via `std::io::stderr().is_terminal()`.
2. **If `path` is `None`:** writer = stderr, auto-detect color.
3. **If `disable_color` is true:** override auto-detected color to `false`.
4. **Lock the global mutex** and replace `LoggerInner` fields atomically.

### Shutdown

No explicit shutdown. `main()` returns, `Box<dyn Write>` writers are dropped,
flushing any buffered output. File handles from `OpenOptions` are closed on drop.

---

## 4. Module docs

### 4.1 `src/main.rs`

**Responsibility:** Program entrypoint. Orchestrates startup.

**Callers:** None (binary entrypoint).

**Called modules:** `cli`, `log`.

**Outgoing calls:**

- `cli::Args::parse()` — clap derive, parses `std::env::args_os()`
- `log::init(...)` — configure global logger state
- `log_info!(...)` — macro, expands to `__log_impl`

**Notes:** If `clap` encounters invalid arguments (e.g. unknown flag, bad log-level
value), `parse()` calls `process::exit(2)` after printing an error — this is clap's
default behaviour and there is no custom error handling.

### 4.2 `src/cli.rs`

**Responsibility:** Define and parse CLI arguments.

**Callers:** `main.rs`

**Exported items:**

- `Args` — struct with clap `#[derive(Parser)]`
  - `repo_path: PathBuf` — positional arg, `#[arg(default_value = ".")]`
  - `repo: Option<PathBuf>` — `#[arg(long, short)]`, overrides positional
  - `log_file: Option<PathBuf>` — `#[arg(long)]`
  - `log_level: Level` — `#[arg(long, short, default_value = "info")]`
  - `no_color: bool` — `#[arg(long)]`
  - `no_timestamp: bool` — `#[arg(long)]`
  - `no_source: bool` — `#[arg(long)]`
- `Args::effective_repo_path()` — returns `self.repo.clone().unwrap_or_else(|| self.repo_path.clone())`

**Dependencies:** `clap::Parser` derive, `crate::log::Level`

**Interaction with log::Level:** The `log_level` field uses `Level` as its type.
clap derives `ValueEnum` on `Level` in `log.rs`, enabling automatic CLI parsing
of level strings (case-insensitive: `info`, `DEBUG`, `Warn` all work).

### 4.3 `src/log.rs`

**Responsibility:** Global thread-safe logger.

**Callers:** `main.rs`, any module that uses the exported macros.

**Internal structure:**

```rust
struct LoggerInner {
    level:     Level,                    // minimum severity to emit
    writer:    Box<dyn Write + Send>,    // output sink (File or Stderr)
    use_color: bool,                     // emit ANSI escape codes
    show_ts:   bool,                     // prepend [HH:MM:SS.ffffff]
    show_src:  bool,                     // append [file:line]
}

static LOGGER: OnceLock<Mutex<LoggerInner>> = OnceLock::new();
```

**Initialisation:** `OnceLock::get_or_init()` creates the default logger on first
access. Default: stderr, Level::Info, color=TTY-detect, ts+src on.

**Locking:** `std::sync::Mutex` guards all reads and writes to `LoggerInner`.
The lock is held for the duration of `__log_impl()` — critical section spans
the full format-and-write cycle to prevent interleaved output from concurrent
callers.

**Level ordering:** `#[derive(PartialOrd, Ord)]` on `#[repr(u8)]` — higher numeric
value = lower priority. Suppression check: `if level > inner.level { return; }`.

**Format pipeline** (inside `__log_impl`, while holding the lock):

```
1. Level check          if level > inner.level → return early
2. Timestamp (if show_ts)
   ├─ Dim-on (color)    "\x1b[2m"
   ├─ Format             UTC from SystemTime → "[HH:MM:SS.ffffff] "
   └─ Dim-off (color)   "\x1b[0m"
3. Level label
   ├─ Color mode:       "<emoji> [<ansi_color>LEVEL\x1b[0m] "
   └─ No-color mode:    "[LEVEL ]"
4. Source (if show_src)
   ├─ Dim-on (color)    "\x1b[2m"
   ├─ Format             "[file:line] "
   └─ Dim-off (color)   "\x1b[0m"
5. Message              format_args
6. Newline (if newline) "\n"
7. Flush                no explicit flush; stdio line-buffered, File fully buffered
```

**Timestamp calculation** (`format_timestamp()`):

```rust
SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()     // system clock failure → epoch
// secs % 86400 → hours/minutes/secs, subsec_micros → microseconds
```

Uses **UTC** (no `localtime_r` equivalent without `chrono` or `libc`).

**Macro expansion:**

```rust
log_info!("port {}", 8080)
// expands to:
$crate::log!($crate::log::Level::Info, "port {}", 8080)
// which expands to:
$crate::log::__log_impl(
    $crate::log::Level::Info,
    file!(),                         // compile-time &str
    line!(),                         // compile-time u32
    format_args!("port {}", 8080),   // opaque fmt::Arguments
    true,                            // newline
)
```

**Color / emoji table:**

| Level | ANSI code  | Emoji | Label |
| ----- | ---------- | ----- | ----- |
| Fatal | `1;34` blu | 💀    | FATAL |
| Error | `1;31` red | 🚨    | ERROR |
| Warn  | `1;33` yel | ⚠️     | WARN  |
| Info  | `1;32` grn | ℹ️     | INFO  |
| Debug | `1;36` cyn | 🛠️     | DEBUG |
| Trace | `1;35` mag | 🔬    | TRACE |

**Private items:**

- `LoggerInner` — struct, not exported
- `LOGGER` — static `OnceLock`, not exported
- `logger()` — returns `&'static Mutex<LoggerInner>`, not exported
- `format_timestamp()` — pure function, not exported

---

## 5. Data flow

```
argv
  │
  ▼
cli::Args::parse()         ──→ repo, log_level, log_file, flags
  │
  ▼
log::init(...)              ──→ mutates global LOGGER static
  │
  ▼
log_info!("...", repo)      ──→ Mutex::lock()
                                  │
                                  ├─ format timestamp string (heap alloc)
                                  ├─ format color codes (stack)
                                  ├─ write! → writer (File / Stderr)
                                  │
                                  └─ Mutex::unlock()
  │
  ▼
main() returns              ──→ writer dropped, file closed
```

---

## 6. Build pipeline

```sh
cargo build
```

1. **Resolution:** `Cargo.toml` declares `clap` with `derive` feature. Resolver
   fetches clap 4.6.4 + transitive deps (clap_builder, clap_derive, anstream,
   anstyle, heck, proc-macro2, quote, syn, unicode-ident, etc.).
2. **Compilation:** Rust 1.96.0 (edition 2024). `clap_derive` proc-macro generates
   `Parser`, `ValueEnum` impls at compile time.
3. **Artifacts:** `target/debug/gitforge` — single binary, dynamically linked to
   system libc. No libgit2, no OpenSSL, no C library beyond the C runtime.

There is no `[profile.release]` override yet — release builds use Cargo defaults
(no LTO, codegen-units=16).

---

## 7. Runtime model

- **Single thread.** No async runtime (`tokio` not present). The process runs in
  `main()` and exits immediately after the log message.
- **Memory:** trivial — one `Mutex<LoggerInner>` in static storage, one
  `Box<dyn Write>` (either `File` or `Stderr`) inside it, one heap-allocated
  timestamp string per log call.
- **Concurrency:** The logger is `Send + Sync` safe (via `Mutex`), but there are
  no concurrent callers at HEAD.

---

## 8. Error propagation

There is no error-handling framework at HEAD:

- **Invalid CLI args:** clap's `parse()` panics (via `process::exit` after
  printing usage). No custom `Result` type.
- **Log file open failure:** `log::init()` silently falls back to stderr and
  writes a warning. The `std::io::Error` from `OpenOptions::open()` is discarded.
- **Mutex poison:** `logger().lock().unwrap()` will panic if any thread panicked
  while holding the lock. At HEAD this is unreachable (single-threaded).
- **System clock failure:** `format_timestamp()` uses `unwrap_or_default()` —
  a broken system clock yields epoch timestamps silently.

---

## 9. Logging architecture

Designed after the C logger at `dotfiles/global/c-cpp-template/c_min_with_make/src/log.c`:

**C feature → Rust equivalent:**

| C                                       | Rust                                   |
| --------------------------------------- | -------------------------------------- |
| `pthread_mutex_t`                       | `std::sync::Mutex`                     |
| `pthread_mutex_init`                    | `Mutex::new()`                         |
| `FILE *` + `fprintf`                    | `Box<dyn Write + Send>` + `write!`     |
| `isatty()`                              | `std::io::IsTerminal::is_terminal()`   |
| `clock_gettime` + `localtime_r`         | `SystemTime` (UTC)                     |
| `__FILE__`, `__LINE__`, `__func__`      | `file!()`, `line!()` (no `__func__`)   |
| `LOG_FATAL(...)` macro                  | `log_fatal!(...)` macro                |
| compile-time `LOG_SHOW_TIME_STAMP`      | runtime `show_ts` bool                 |
| compile-time `LOG_SHOW_SOURCE_LOCATION` | runtime `show_src` bool                |
| `LOG_PERROR`                            | not ported (Rust doesn't have `errno`) |

**Not ported from C:**

- `LOG_PERROR` — no Rust equivalent of C's `perror`/`strerror(errno)`; callers
  should use `log_error!("msg: {}", err)` with `Display`/`Debug`.
- `__func__` — Rust has no stable function-name macro; `file!() + line!()` is used
  instead.
- local-time timestamps — C uses `localtime_r`; Rust version uses UTC to avoid
  the `chrono` or `libc` dependency.

---

## 10. External dependencies

| Crate          | Why it exists                           | Transitive deps count |
| -------------- | --------------------------------------- | --------------------- |
| `clap` (4.6.4) | CLI argument parsing with derive macros | 20 transitive crates  |

No other dependencies. The logger is built on `std` only.

---

## 11. Known limitations (observable in HEAD)

1. **No MCP protocol.** The server advertises no capabilities and handles no
   requests. It is a CLI binary, not an MCP server.
2. **No Git operations.** `git2` is not in `Cargo.toml`. No repository inspection
   or manipulation functions exist.
3. **No async runtime.** `tokio` is not present. No non-blocking I/O.
4. **Immediate exit.** The program logs one line and exits. It does not enter a
   server loop and cannot receive or respond to messages.
5. **UTC-only timestamps.** The logger emits `[HH:MM:SS.ffffff]` in UTC, not
   local time. The C original used `localtime_r`. Adding local-time support
   requires `chrono` or `libc`.
6. **No buffered file I/O.** `OpenOptions::new().append(true).create(true).open()`
   opens with OS default buffering. No explicit flush is called on `File` writers
   (unlike the C original which called `fflush`). `Stderr` is line-buffered by
   the stdlib; `File` is fully buffered — log lines to a file may not appear
   immediately on crash.
7. **Mutex poisoning.** A single `unwrap()` on the lock — if a thread panics
   while holding the logger mutex, all subsequent logging will panic.
8. **No tests.** Zero test coverage across all modules.
