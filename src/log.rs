use clap::ValueEnum;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
#[cfg(feature = "log-timestamp")]
use std::time::SystemTime;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[clap(rename_all = "lowercase")]
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
}

static LOGGER: OnceLock<Mutex<LoggerInner>> = OnceLock::new();

fn logger() -> &'static Mutex<LoggerInner> {
	LOGGER.get_or_init(|| {
		Mutex::new(LoggerInner {
			level: Level::Info,
			writer: Box::new(std::io::stderr()),
			use_color: std::io::stderr().is_terminal(),
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
				(
					Box::new(std::io::stderr()) as Box<dyn Write + Send>,
					std::io::stderr().is_terminal(),
				)
			}
		},
		None => (
			Box::new(std::io::stderr()) as Box<dyn Write + Send>,
			std::io::stderr().is_terminal(),
		),
	};

	let mut inner = logger().lock().unwrap();
	inner.writer = writer;
	inner.use_color = use_color;
	inner.level = level;
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

pub fn set_color(v: bool) {
	logger().lock().unwrap().use_color = v;
}

#[cfg(feature = "log-timestamp")]
fn is_leap(y: i64) -> bool {
	(y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(feature = "log-timestamp")]
fn days_to_date(days: u64) -> (u64, u64, u64) {
	let mut y: i64 = 1970;
	let mut d = days as i64;

	loop {
		let days_in_year = if is_leap(y) { 366 } else { 365 };
		if d < days_in_year {
			break;
		}
		d -= days_in_year;
		y += 1;
	}

	let months = [
		31,
		if is_leap(y) { 29 } else { 28 },
		31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
	];
	let mut m = 1;
	for &md in &months {
		if d < md {
			break;
		}
		d -= md;
		m += 1;
	}

	(y as u64, m, (d + 1) as u64)
}

#[cfg(feature = "log-timestamp")]
fn format_timestamp() -> String {
	let dur = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap_or_default();
	let total_secs = dur.as_secs();
	let days = total_secs / 86400;
	let remaining = total_secs % 86400;
	let h = remaining / 3600;
	let m = (remaining % 3600) / 60;
	let s = remaining % 60;
	let us = dur.subsec_micros();
	let (y, mon, d) = days_to_date(days);
	let mon_abbr = match mon {
		1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr", 5 => "May", 6 => "Jun",
		7 => "Jul", 8 => "Aug", 9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
		_ => "???",
	};
	format!("[{:02}-{}-{} {:02}:{:02}:{:02}.{:06}] ", d, mon_abbr, y, h, m, s, us)
}

pub fn __log_impl(level: Level, _file: &str, _line: u32, args: fmt::Arguments<'_>, newline: bool) {
	let mut inner = logger().lock().unwrap();
	if level > inner.level {
		return;
	}

	let color = inner.use_color;

	#[cfg(feature = "log-timestamp")]
	{
		if color {
			let _ = write!(inner.writer, "\x1b[2m");
		}
		let _ = write!(inner.writer, "{}", format_timestamp());
		if color {
			let _ = write!(inner.writer, "\x1b[0m");
		}
	}

	if color {
		let _ = write!(inner.writer, "{} [", level.emoji());
		let _ = write!(
			inner.writer,
			"{}{}{}",
			level.ansi_color(),
			level.label(),
			"\x1b[0m"
		);
		let _ = write!(inner.writer, "] ");
	} else {
		let _ = write!(inner.writer, "[{:<5}] ", level.label());
	}

	#[cfg(feature = "log-source")]
	{
		if color {
			let _ = write!(inner.writer, "\x1b[2m");
		}
		let _ = write!(inner.writer, "[{}:{}] ", _file, _line);
		if color {
			let _ = write!(inner.writer, "\x1b[0m");
		}
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
