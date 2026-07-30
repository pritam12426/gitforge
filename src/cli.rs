use std::path::PathBuf;

use clap::Parser;

use crate::logging::{LogFormat, LogLevel};

#[derive(Parser)]
#[command(name = "gitforge", about = "Git MCP server", color = clap::ColorChoice::Auto)]
pub struct Cli {
	/// Path to the Git repository (defaults to current directory).
	/// Can also be set via the GITFORGE_REPO environment variable.
	#[arg(long = "repo", default_value = ".", env = "GITFORGE_REPO")]
	pub repo_path: PathBuf,

	/// Restrict tool operations to this directory tree.
	/// Defaults to --repo if not set. Can also be set via
	/// the GITFORGE_ALLOWED_REPO environment variable.
	#[arg(long, env = "GITFORGE_ALLOWED_REPO")]
	pub allowed_repo: Option<PathBuf>,

	/// Path to log file (defaults to stderr).
	#[arg(long)]
	pub log_file: Option<PathBuf>,

	/// Minimum log level. Overridden at runtime by GITFORGE_LOG_LEVEL
	/// if that env var is set.
	#[arg(long, short, default_value = "warn")]
	pub log_level: LogLevel,

	/// Log line format.
	#[arg(long, default_value = "pretty")]
	pub log_format: LogFormat,
}
