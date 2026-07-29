use clap::Parser;

use gitforge::cli::Cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();
	gitforge::run(cli)
}
