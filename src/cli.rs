use std::path::PathBuf;

use clap::Parser;

use crate::logging::{LogFormat, LogLevel};

#[derive(Parser)]
#[command(name = "gitforge", about = "Git MCP server", color = clap::ColorChoice::Auto)]
pub struct Cli {
	/// Path to the Git repository (defaults to current directory).
	#[arg(long = "repo", default_value = ".")]
	pub repo_path: PathBuf,

	/// Path to log file (defaults to stderr).
	#[arg(long)]
	pub log_file: Option<PathBuf>,

	/// Minimum log level. Overridden at runtime by GITFORGE_LOG_LEVEL
	/// if that env var is set.
	#[arg(long, short, default_value = "info")]
	pub log_level: LogLevel,

	/// Log line format.
	#[arg(long, default_value = "pretty")]
	pub log_format: LogFormat,
}
