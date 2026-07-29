//! gitforge as a library, with `src/main.rs` reduced to a thin wrapper
//! around [`run`].
//!
//! Splitting a lib out from what was previously a binary-only crate is
//! the one structural change here that isn't visible from the outside:
//! it exists so `tests/integration.rs` isn't limited to spawning the
//! compiled binary and talking to it over stdio pipes (still done, and
//! still the primary way the *protocol* itself is tested) — it can also
//! reach [`transport::http::build_app`] directly and drive the HTTP
//! transport in-process with `tower::ServiceExt::oneshot`, with no real
//! socket involved.

pub mod cli;
pub mod error;
pub mod git;
pub mod logging;
pub mod mcp;
pub mod tools;
pub mod transport;

use std::sync::Arc;

use cli::{Cli, Command};

/// Builds the router (repo actor + all registered tools) and runs
/// whichever transport the CLI selected. This is the entire body of
/// `main`, factored out so it's usable from integration tests too.
pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
	logging::log_init(cli.log_file.as_deref().and_then(|p| p.to_str()), cli.log_level, cli.log_format);

	log_info!("gitforge starting — repo: {}", cli.repo_path.display());

	let repo = git::RepoHandle::spawn(&cli.repo_path)?;
	let mut router = mcp::Router::new(repo.clone());
	tools::register_all(&mut router, repo);

	match cli.command {
		None => {
			log_info!("gitforge: running in stdio mode");
			transport::stdio::run(&router)?;
		}
		Some(Command::Server { host, port }) => {
			log_info!("gitforge: running in http mode on {}:{}", host, port);
			// Stdio mode stays fully synchronous — no tokio runtime is
			// ever created unless `server` mode is actually requested.
			let rt = tokio::runtime::Builder::new_multi_thread()
				.enable_all()
				.build()
				.map_err(error::GitforgeError::Io)?;
			rt.block_on(transport::http::run(Arc::new(router), &host, port))?;
		}
	}

	Ok(())
}
