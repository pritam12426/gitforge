mod cli;
mod error;
mod git;
mod log;
mod mcp;
mod tools;

use clap::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = cli::Args::parse();
	let repo_path = args.effective_repo_path();

	log::init(args.log_file.as_deref(), args.log_level);

	log_info!("gitforge starting — repo: {}", repo_path.display());

	let repo = git::RepoHandle::spawn(&repo_path)?;
	let mut router = mcp::Router::new();

	tools::register_all(&mut router, repo);

	mcp::transport::run_stdio_loop(&router)?;

	Ok(())
}
