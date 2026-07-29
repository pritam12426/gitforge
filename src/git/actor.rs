use std::sync::mpsc::Receiver;

use crate::error::GitforgeError;
use crate::{log_debug, log_error, log_info, log_trace};

use super::commands::{RepoCommand, RepoResponse, Respond};
use super::ops;

/// The actor's main loop. Owns the one `git2::Repository` for the
/// process's lifetime and processes commands strictly one at a time,
/// which is what makes it safe to hand out `RepoHandle` clones freely
/// from both the stdio loop and (many, concurrent) HTTP handlers.
pub fn run(repo: git2::Repository, receiver: Receiver<RepoCommand>) {
	log_info!("git actor: started, waiting for commands");
	while let Ok(cmd) = receiver.recv() {
		dispatch(&repo, cmd);
	}
	log_info!("git actor: channel closed, shutting down");
}

fn dispatch(repo: &git2::Repository, cmd: RepoCommand) {
	let name = cmd.name();
	match cmd {
		RepoCommand::CheckHealth { respond } => {
			run_and_respond(name, respond, || Ok(RepoResponse::Health));
		}
		RepoCommand::GetStatus { respond } => {
			run_and_respond(name, respond, || ops::status::run(repo));
		}
		RepoCommand::GetLog { offset, max_count, respond } => {
			run_and_respond(name, respond, || ops::log::run(repo, offset, max_count));
		}
		RepoCommand::GetBranches { respond } => {
			run_and_respond(name, respond, || ops::branches::run(repo));
		}
		RepoCommand::GetDiff { respond } => {
			run_and_respond(name, respond, || ops::diff::run(repo));
		}
		RepoCommand::ShowCommit { revision, respond } => {
			run_and_respond(name, respond, || ops::show::run(repo, &revision));
		}
		RepoCommand::CreateCommit { message, author_name, author_email, respond } => {
			run_and_respond(name, respond, || {
				ops::commit::run(repo, &message, &author_name, &author_email)
			});
		}
		RepoCommand::StageFiles { paths, respond } => {
			run_and_respond(name, respond, || ops::stage::run(repo, &paths));
		}
		RepoCommand::CreateBranch { name: branch_name, revision, respond } => {
			run_and_respond(name, respond, || ops::branch_create::run(repo, &branch_name, &revision));
		}
		RepoCommand::Checkout { branch, respond } => {
			run_and_respond(name, respond, || ops::checkout::run(repo, &branch));
		}
		RepoCommand::Merge { branch, respond } => {
			run_and_respond(name, respond, || ops::merge::run(repo, &branch));
		}
	}
}

/// Wraps a single actor operation with uniform lifecycle logging:
/// received -> processing -> completed/errored. This is what satisfies
/// "every actor command should log its lifecycle" without repeating the
/// same four log lines inside every `ops::*::run` function.
fn run_and_respond<F>(op_name: &'static str, respond: Respond, f: F)
where
	F: FnOnce() -> Result<RepoResponse, GitforgeError>,
{
	log_debug!("actor: '{}' received", op_name);
	log_trace!("actor: '{}' processing", op_name);

	let result = f();

	match &result {
		// Debug-format the response but cap it — a Diff/ShowCommit
		// payload can be many KB and would otherwise dominate the logs.
		Ok(resp) => log_info!(
			"actor: '{}' completed -> {}",
			op_name,
			crate::logging::truncate_for_log(&format!("{:?}", resp), 200)
		),
		Err(e) => log_error!("actor: '{}' errored: {}", op_name, e),
	}

	if respond.send(result).is_err() {
		log_error!("actor: '{}' — caller dropped the response channel", op_name);
	}
}
