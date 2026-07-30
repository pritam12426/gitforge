pub mod cli;
pub mod error;
pub mod git;
pub mod logging;
pub mod mcp;
pub mod tools;
pub mod transport;

pub fn run(cli: cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
	logging::log_init(cli.log_file.as_deref().and_then(|p| p.to_str()), cli.log_level, cli.log_format);

	log_info!("gitforge starting — repo: {}", cli.repo_path.display());

	let repo = git::RepoHandle::spawn(&cli.repo_path).inspect_err(|e| {
		log_fatal!("not a git repository at '{}': {e}", cli.repo_path.display());
	})?;
	let mut router = mcp::Router::new(repo.clone(), cli.allowed_repo);
	tools::register_all(&mut router, repo);

	transport::stdio::run(&router)?;
	Ok(())
}
