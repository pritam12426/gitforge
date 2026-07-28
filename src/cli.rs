use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "gitforge", about = "Git MCP server")]
pub struct Args {
	/// Path to the Git repository (defaults to current directory)
	#[arg(default_value = ".")]
	pub repo_path: PathBuf,

	/// Explicit repository path (overrides positional argument)
	#[arg(long, short)]
	pub repo: Option<PathBuf>,

	/// Path to log file (defaults to stderr)
	#[arg(long)]
	pub log_file: Option<PathBuf>,

	/// Minimum log level [off, fatal, error, warn, info, debug, trace]
	#[arg(long, short, default_value = "info")]
	pub log_level: crate::log::Level,

	/// Disable ANSI color output
	#[arg(long)]
	pub no_color: bool,

	/// Suppress timestamps in log output
	#[arg(long)]
	pub no_timestamp: bool,

	/// Suppress source location [file:line] in log output
	#[arg(long)]
	pub no_source: bool,
}

impl Args {
	pub fn effective_repo_path(&self) -> PathBuf {
		self.repo.clone().unwrap_or_else(|| self.repo_path.clone())
	}
}
