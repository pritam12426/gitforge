use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::logging::{LogFormat, LogLevel};

#[derive(Parser)]
#[command(name = "gitforge", about = "Git MCP server", color = clap::ColorChoice::Auto)]
pub struct Cli {
	/// Path to the Git repository (defaults to current directory).
	/// Applies to both stdio mode and `server` mode.
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

	#[command(subcommand)]
	pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
	/// Serve the same MCP protocol over HTTP (JSON-RPC POST) instead of
	/// stdio. Example: `gitforge server --host 0.0.0.0 -P 8787`.
	Server {
		/// Address to bind to.
		#[arg(long, default_value = "127.0.0.1")]
		host: String,

		/// Port to bind to.
		#[arg(long, short = 'P', default_value_t = 8787)]
		port: u16,
	},
}
