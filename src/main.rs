mod cli;
mod log;

use clap::Parser;

fn main() {
	let args = cli::Args::parse();
	let repo = args.effective_repo_path();

	log::init(
		args.log_file.as_deref(),
		args.log_level,
		!args.no_timestamp,
		!args.no_source,
		args.no_color,
	);

	log_info!("gitforge starting — repo: {}", repo.display());
}
