//! rust_log — thread-safe logging, ported from a C implementation.
//!
//! ## Compile-time configuration
//!
//! C used `#ifdef` fed by `-D` flags in the Makefile. Rust has no
//! preprocessor, so the same "decide at compile time, pay zero cost if
//! disabled" behaviour is done with **Cargo features** instead. Code gated
//! behind a disabled feature is not merely skipped at runtime — it is not
//! compiled into the binary at all, exactly like the C `#ifdef` blocks.
//!
//! Turn these on in `Cargo.toml` (or via `--features` on the CLI):
//!
//!   * `show_time_stamp`      — equivalent of `-DLOG_SHOW_TIME_STAMP`
//!   * `show_source_location` — equivalent of `-DLOG_SHOW_SOURCE_LOCATION`
//!
//! Example:
//! ```text
//! cargo build --features "show_time_stamp show_source_location"
//! ```
//!
//! ## Usage
//! ```no_run
//! use rust_log::{log_init, LogLevel};
//!
//! log_init(None, LogLevel::Debug);
//! rust_log::log_info!("server started on port {}", 8080);
//! ```

use clap::ValueEnum;
use std::fs::{File, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "show_time_stamp")]
use chrono::Local;

// ── ANSI colour codes ───────────────────────────────────────────────────────
mod color {
	pub const RESET:        &str = "\x1b[0m";
	pub const BOLD_RED:     &str = "\x1b[1;31m";
	pub const BOLD_GREEN:   &str = "\x1b[1;32m";
	pub const BOLD_YELLOW:  &str = "\x1b[1;33m";
	pub const BOLD_BLUE:    &str = "\x1b[1;34m";
	pub const BOLD_MAGENTA: &str = "\x1b[1;35m";
	pub const BOLD_CYAN:    &str = "\x1b[1;36m";
	pub const DIM:          &str = "\x1b[2m";
}

// ── Log levels (lower number = higher priority, same as the C enum) ────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[clap(rename_all = "lowercase")]
#[repr(i32)]
pub enum LogLevel {
	Off   = 0,
	Fatal = 1,
	Error = 2,
	Warn  = 3,
	Info  = 4,
	Debug = 5,
	Trace = 6,
}

// ── Output stream: either stderr or an opened file ─────────────────────────
enum Output {
	Stderr,
	File(File),
}

impl Write for Output {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match self {
			Output::Stderr => io::stderr().write(buf),
			Output::File(f) => f.write(buf),
		}
	}
	fn flush(&mut self) -> io::Result<()> {
		match self {
			Output::Stderr => io::stderr().flush(),
			Output::File(f) => f.flush(),
		}
	}
}

/// A cloned handle to the logger's current output stream, returned by
/// [`log_get_file`]. Mirrors `log_get_file()` in the C API, which returned
/// the raw `FILE *`; Rust can't hand out an aliased `File`/stderr handle
/// directly, so this returns an independent, freshly-cloned writer instead.
pub enum LogFile {
	Stderr,
	File(File),
}

impl Write for LogFile {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match self {
			LogFile::Stderr => io::stderr().write(buf),
			LogFile::File(f) => f.write(buf),
		}
	}
	fn flush(&mut self) -> io::Result<()> {
		match self {
			LogFile::Stderr => io::stderr().flush(),
			LogFile::File(f) => f.flush(),
		}
	}
}

// ── Logger state, protected by a single mutex (same tradeoff the C
//    comment calls out: log_record() always needs exclusive access to
//    avoid interleaved output, so a plain Mutex beats an RwLock here) ──────
struct LoggerState {
	stream: Option<Output>,
	level: LogLevel,
	use_color: bool,
}

static LOGGER: OnceLock<Mutex<LoggerState>> = OnceLock::new();

fn logger() -> &'static Mutex<LoggerState> {
	LOGGER.get_or_init(|| {
		Mutex::new(LoggerState {
			stream: None,
			level: LogLevel::Info,
			use_color: false,
		})
	})
}

// ── Public API ───────────────────────────────────────────────────────────

/// Initialise the logger. Thread-safe; may be called multiple times.
///
/// * `file_path` — log file path, or `None` for stderr (colour is
///   auto-disabled when writing to a file).
/// * `level` — minimum severity to emit (e.g. `LogLevel::Info`).
pub fn log_init(file_path: Option<&str>, level: LogLevel) {
	// Resolve the new stream and colour flag BEFORE taking the lock, so
	// the lock is held for the shortest possible time (same rationale as
	// the C version).
	let (stream, use_color) = match file_path {
		None => (Output::Stderr, io::stderr().is_terminal()),
		Some(path) => match OpenOptions::new().create(true).append(true).open(path) {
			Ok(f) => (Output::File(f), false),
			Err(_) => {
				eprintln!(
					"[LOG] warning: could not open log file '{}', falling back to stderr",
					path
				);
				(Output::Stderr, io::stderr().is_terminal())
			}
		},
	};

	let mut state = logger().lock().unwrap();
	state.stream = Some(stream);
	state.use_color = use_color;
	state.level = level;
}

/// Set the minimum log level; messages below this are suppressed.
pub fn log_set_level(level: LogLevel) {
	logger().lock().unwrap().level = level;
}

/// Get the current minimum log level.
pub fn log_get_level() -> LogLevel {
	logger().lock().unwrap().level
}

/// Check whether ANSI colour is currently enabled.
pub fn log_use_color() -> bool {
	logger().lock().unwrap().use_color
}

/// Get a fresh handle to the current log output (stderr if not yet
/// initialised). See [`LogFile`] for why this differs slightly from the
/// C signature.
pub fn log_get_file() -> io::Result<LogFile> {
	let state = logger().lock().unwrap();
	match &state.stream {
		None | Some(Output::Stderr) => Ok(LogFile::Stderr),
		Some(Output::File(f)) => Ok(LogFile::File(f.try_clone()?)),
	}
}

fn level_label_plain(level: LogLevel) -> &'static str {
	match level {
		LogLevel::Fatal => "[FATAL] ",
		LogLevel::Error => "[ERROR] ",
		LogLevel::Warn  => "[WARN ] ",
		LogLevel::Info  => "[INFO ] ",
		LogLevel::Debug => "[DEBUG] ",
		LogLevel::Trace => "[TRACE] ",
		LogLevel::Off   => "[UNKWN] ",
	}
}

fn write_color_label(out: &mut dyn Write, level: LogLevel) -> io::Result<()> {
	use color::*;
	match level {
		LogLevel::Fatal => write!(out, "\u{1F480} [{BOLD_BLUE}FATAL{RESET}] "),
		LogLevel::Error => write!(out, "\u{1F6A8} [{BOLD_RED}ERROR{RESET}] "),
		LogLevel::Warn => write!(out, "\u{26A0}\u{FE0F}  [{BOLD_YELLOW}WARN {RESET}] "),
		LogLevel::Info => write!(out, "\u{2139}\u{FE0F}  [{BOLD_GREEN}INFO {RESET}] "),
		LogLevel::Debug => write!(out, "\u{1F6E0}\u{FE0F}  [{BOLD_CYAN}DEBUG{RESET}] "),
		LogLevel::Trace => write!(out, "\u{1F52C} [{BOLD_MAGENTA}TRACE{RESET}] "),
		LogLevel::Off => write!(out, "[{BOLD_BLUE}UNKWN{RESET}] "),
	}
}

#[cfg(feature = "show_time_stamp")]
fn write_time_stamp(out: &mut dyn Write, use_color: bool) -> io::Result<()> {
	// chrono gives us real local-time + microsecond precision, matching
	// clock_gettime(CLOCK_REALTIME) + localtime_r() in the C version.
	let now = Local::now();
	if use_color {
		write!(out, "{}", color::DIM)?;
	}
	write!(out, "[{}] ", now.format("%d-%b-%Y %H:%M:%S%.6f"))?;
	if use_color {
		write!(out, "{}", color::RESET)?;
	}
	Ok(())
}

/// Source-location info captured at the call site. Only ever constructed
/// when the `show_source_location` feature is enabled — see the
/// `__loc!()` helper macro below.
#[doc(hidden)]
pub struct SourceLoc {
	pub file: &'static str,
	pub line: u32,
	pub func: &'static str,
}

/// Core logging function: formats and writes a log message.
/// Called by the `log_*!` macros — prefer those over calling this directly.
#[doc(hidden)]
pub fn log_record(level: LogLevel, loc: Option<SourceLoc>, new_line: bool, msg: &str) {
	let mut state = logger().lock().unwrap();

	if state.stream.is_none() {
		#[cfg(feature = "show_source_location")]
		if let Some(l) = &loc {
			let _ = write!(
				io::stderr(),
				"{}[{}:{}:{}]{} ",
				color::DIM,
				l.file,
				l.line,
				l.func,
				color::RESET
			);
		}
		let _ = write!(
			io::stderr(),
			"{}[LOG] error: log_init() not called — dropping message{}",
			color::BOLD_RED,
			color::RESET
		);
		if new_line {
			let _ = writeln!(io::stderr());
		}
		return;
	}

	// Suppress messages below the configured level.
	if (level as i32) > (state.level as i32) {
		return;
	}

	let use_color = state.use_color;
	let stream = state.stream.as_mut().unwrap();

	#[cfg(feature = "show_time_stamp")]
	let _ = write_time_stamp(stream, use_color);

	let _ = if use_color {
		write_color_label(stream, level)
	} else {
		write!(stream, "{}", level_label_plain(level))
	};

	#[cfg(feature = "show_source_location")]
	if let Some(l) = &loc {
		let (pre, post) = if use_color {
			(color::DIM, color::RESET)
		} else {
			("", "")
		};
		let _ = write!(stream, "{}[{}:{}:{}]{} ", pre, l.file, l.line, l.func, post);
	}
	#[cfg(not(feature = "show_source_location"))]
	let _ = &loc; // silence "unused" when the feature is off

	let _ = write!(stream, "{}", msg);
	if new_line {
		let _ = writeln!(stream);
	}
	let _ = stream.flush();
}

// ── Macros ───────────────────────────────────────────────────────────────
//
// C captured __FILE__/__LINE__/__func__ automatically via preprocessor
// macros. Rust's file!()/line!() are the direct equivalents; a function's
// *name* has no built-in macro, so `__function_name!()` uses the common
// `std::any::type_name` trick to recover it at compile time, at zero
// runtime cost.

#[doc(hidden)]
#[macro_export]
macro_rules! __function_name {
	() => {{
		fn f() {}
		fn type_name_of<T>(_: T) -> &'static str {
			std::any::type_name::<T>()
		}
		let name = type_name_of(f);
		&name[..name.len() - 3]
	}};
}

#[cfg(feature = "show_source_location")]
#[doc(hidden)]
#[macro_export]
macro_rules! __loc {
	() => {
		Some($crate::log::SourceLoc {
			file: file!(),
			line: line!(),
			func: $crate::__function_name!(),
		})
	};
}

#[cfg(not(feature = "show_source_location"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __loc {
	() => {
		None
	};
}

/// Log with custom newline behaviour (`false` = no newline, `true` = with
/// newline). Used internally; prefer `log_fatal!` / `log_error!` / etc.
#[macro_export]
macro_rules! log_custom {
    ($level:expr, $newline:expr, $($arg:tt)*) => {{
        $crate::log::log_record($level, $crate::__loc!(), $newline, &format!($($arg)*));
    }};
}

/// Log an error and append the last OS error (equivalent of `perror()`).
#[macro_export]
macro_rules! log_perror {
    ($($arg:tt)*) => {{
        $crate::log_custom!($crate::log::LogLevel::Error, false, $($arg)*);
        eprintln!(" {}", ::std::io::Error::last_os_error());
    }};
}

#[macro_export]
macro_rules! log_fatal {
    ($($arg:tt)*) => { $crate::log_custom!($crate::log::LogLevel::Fatal, true, $($arg)*) };
}
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::log_custom!($crate::log::LogLevel::Error, true, $($arg)*) };
}
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::log_custom!($crate::log::LogLevel::Warn, true, $($arg)*) };
}
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::log_custom!($crate::log::LogLevel::Info, true, $($arg)*) };
}
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => { $crate::log_custom!($crate::log::LogLevel::Debug, true, $($arg)*) };
}
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => { $crate::log_custom!($crate::log::LogLevel::Trace, true, $($arg)*) };
}

/// Check if messages at the given level would be emitted right now.
#[macro_export]
macro_rules! log_level_is_enabled {
	($level:expr) => {
		$crate::log::log_get_level() >= $level
	};
}
