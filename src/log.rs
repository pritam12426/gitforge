use std::fmt;
use std::fs::OpenOptions;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
	Off = 0,
	Fatal = 1,
	Error = 2,
	Warn = 3,
	Info = 4,
	Debug = 5,
	Trace = 6,
}

impl Level {
	fn label(self) -> &'static str {
		match self {
			Level::Off => "OFF",
			Level::Fatal => "FATAL",
			Level::Error => "ERROR",
			Level::Warn => "WARN",
			Level::Info => "INFO",
			Level::Debug => "DEBUG",
			Level::Trace => "TRACE",
		}
	}

	fn ansi_color(self) -> &'static str {
		match self {
			Level::Fatal => "\x1b[1;34m",
			Level::Error => "\x1b[1;31m",
			Level::Warn => "\x1b[1;33m",
			Level::Info => "\x1b[1;32m",
			Level::Debug => "\x1b[1;36m",
			Level::Trace => "\x1b[1;35m",
			_ => "\x1b[1;37m",
		}
	}

	fn emoji(self) -> &'static str {
		match self {
			Level::Fatal => "\u{1f480}",
			Level::Error => "\u{1f6a8}",
			Level::Warn => "\u{26a0}\u{fe0f}",
			Level::Info => "\u{2139}\u{fe0f}",
			Level::Debug => "\u{1f6e0}\u{fe0f}",
			Level::Trace => "\u{1f52c}",
			_ => "",
		}
	}
}

struct LoggerInner {
	level: Level,
	writer: Box<dyn Write + Send>,
	use_color: bool,
	show_ts: bool,
	show_src: bool,
}

static LOGGER: OnceLock<Mutex<LoggerInner>> = OnceLock::new();

fn logger() -> &'static Mutex<LoggerInner> {
	LOGGER.get_or_init(|| {
		Mutex::new(LoggerInner {
			level: Level::Info,
			writer: Box::new(std::io::stderr()),
			use_color: std::io::stderr().is_terminal(),
			show_ts: true,
			show_src: true,
		})
	})
}

pub fn init(path: Option<&Path>, level: Level) {
	let (writer, use_color) = match path {
		Some(p) => match OpenOptions::new().append(true).create(true).open(p) {
			Ok(f) => (Box::new(f) as Box<dyn Write + Send>, false),
			Err(_) => {
				let _ = write!(
					std::io::stderr().lock(),
					"[LOG] warning: could not open log file '{}', falling back to stderr\n",
					p.display()
				);
				(Box::new(std::io::stderr()) as Box<dyn Write + Send>, std::io::stderr().is_terminal())
			}
		},
		None => (Box::new(std::io::stderr()) as Box<dyn Write + Send>, std::io::stderr().is_terminal()),
	};

	let mut inner = logger().lock().unwrap();
	inner.writer = writer;
	inner.use_color = use_color;
	inner.level = level;
	inner.show_ts = true;
	inner.show_src = true;
}

pub fn set_level(level: Level) {
	logger().lock().unwrap().level = level;
}

pub fn level() -> Level {
	logger().lock().unwrap().level
}

pub fn use_color() -> bool {
	logger().lock().unwrap().use_color
}

fn format_timestamp() -> String {
	let dur = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap_or_default();
	let secs = dur.as_secs() % 86400;
	let h = secs / 3600;
	let m = (secs % 3600) / 60;
	let s = secs % 60;
	let us = dur.subsec_micros();
	format!("[{:02}:{:02}:{:02}.{:06}] ", h, m, s, us)
}

pub fn __log_impl(
	level: Level,
	file: &str,
	line: u32,
	args: fmt::Arguments<'_>,
	newline: bool,
) {
	let mut inner = logger().lock().unwrap();
	if level > inner.level {
		return;
	}

	let color = inner.use_color;

	if inner.show_ts && color {
		let _ = write!(inner.writer, "\x1b[2m");
	}
	if inner.show_ts {
		let _ = write!(inner.writer, "{}", format_timestamp());
	}
	if inner.show_ts && color {
		let _ = write!(inner.writer, "\x1b[0m");
	}

	if color {
		let _ = write!(inner.writer, "{} [", level.emoji());
		let _ = write!(inner.writer, "{}{}{}", level.ansi_color(), level.label(), "\x1b[0m");
		let _ = write!(inner.writer, "] ");
	} else {
		let _ = write!(inner.writer, "[{:<5}] ", level.label());
	}

	if inner.show_src && color {
		let _ = write!(inner.writer, "\x1b[2m");
	}
	if inner.show_src {
		let _ = write!(inner.writer, "[{}:{}] ", file, line);
	}
	if inner.show_src && color {
		let _ = write!(inner.writer, "\x1b[0m");
	}

	let _ = write!(inner.writer, "{}", args);
	if newline {
		let _ = writeln!(inner.writer, "");
	}
}

#[macro_export]
macro_rules! log {
	($level:expr, $($arg:tt)+) => {
		$crate::log::__log_impl($level, file!(), line!(), format_args!($($arg)+), true)
	};
}

#[macro_export]
macro_rules! log_ {
	($level:expr, $($arg:tt)+) => {
		$crate::log::__log_impl($level, file!(), line!(), format_args!($($arg)+), false)
	};
}

#[macro_export]
macro_rules! log_fatal {
	($($arg:tt)+) => { $crate::log!($crate::log::Level::Fatal, $($arg)+) };
}

#[macro_export]
macro_rules! log_error {
	($($arg:tt)+) => { $crate::log!($crate::log::Level::Error, $($arg)+) };
}

#[macro_export]
macro_rules! log_warn {
	($($arg:tt)+) => { $crate::log!($crate::log::Level::Warn, $($arg)+) };
}

#[macro_export]
macro_rules! log_info {
	($($arg:tt)+) => { $crate::log!($crate::log::Level::Info, $($arg)+) };
}

#[macro_export]
macro_rules! log_debug {
	($($arg:tt)+) => { $crate::log!($crate::log::Level::Debug, $($arg)+) };
}

#[macro_export]
macro_rules! log_trace {
	($($arg:tt)+) => { $crate::log!($crate::log::Level::Trace, $($arg)+) };
}
