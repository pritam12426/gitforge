/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use std::sync::mpsc::Receiver;

use crate::error::GitforgeError;
use crate::{log_debug, log_error, log_info, log_trace};

use super::commands::{RepoCommand, RepoResponse, Respond};
use super::ops;

/// The actor's main loop. Owns the one `git2::Repository` for the
/// process's lifetime and processes commands strictly one at a time,
/// which is what makes it safe to hand out `RepoHandle` clones freely
/// from the stdio loop.
///
/// The `git2::Repository` is `!Send + !Sync`, so it must live on
/// exactly one thread.  Every git operation (status, log, diff, ...)
/// is submitted as a `RepoCommand` over an mpsc channel and runs
/// here, serially — no locks needed.
pub fn run(repo: git2::Repository, receiver: Receiver<RepoCommand>) {
	log_info!("git actor: started, waiting for commands");
	while let Ok(cmd) = receiver.recv() {
		dispatch(&repo, cmd);
	}
	log_info!("git actor: channel closed, shutting down");
}

/// Matches a `RepoCommand` to its `ops::*::run` function.  Every arm
/// follows the same pattern through `run_and_respond`, which provides
/// uniform lifecycle logging (received → processing → completed/errored).
///
/// The `repo` reference is shared — commands execute sequentially on
/// this single thread, so there is never contention.
fn dispatch(repo: &git2::Repository, cmd: RepoCommand) {
	let name = cmd.name();
	match cmd {
		RepoCommand::CheckHealth { respond } => {
			run_and_respond(name, respond, || Ok(RepoResponse::Health));
		}
		RepoCommand::GetStatus { respond } => {
			run_and_respond(name, respond, || ops::status::run(repo));
		}
		RepoCommand::GetLog {
			offset,
			max_count,
			respond,
		} => {
			run_and_respond(name, respond, || ops::log::run(repo, offset, max_count));
		}
		RepoCommand::GetBranches {
			branch_type,
			contains,
			not_contains,
			respond,
		} => {
			run_and_respond(name, respond, || {
				ops::branches::run(
					repo,
					&branch_type,
					contains.as_deref(),
					not_contains.as_deref(),
				)
			});
		}
		RepoCommand::GetDiffUnstaged { respond } => {
			run_and_respond(name, respond, || ops::diff_unstaged::run(repo));
		}
		RepoCommand::GetDiffStaged { respond } => {
			run_and_respond(name, respond, || ops::diff_staged::run(repo));
		}
		RepoCommand::GetDiffTarget { target, respond } => {
			run_and_respond(name, respond, || ops::diff_target::run(repo, &target));
		}
		RepoCommand::ShowCommit { revision, respond } => {
			run_and_respond(name, respond, || ops::show::run(repo, &revision));
		}
		RepoCommand::CreateCommit {
			message,
			author_name,
			author_email,
			respond,
		} => {
			run_and_respond(name, respond, || {
				ops::commit::run(repo, &message, &author_name, &author_email)
			});
		}
		RepoCommand::StageFiles { paths, respond } => {
			run_and_respond(name, respond, || ops::stage::run(repo, &paths));
		}
		RepoCommand::CreateBranch {
			name: branch_name,
			revision,
			respond,
		} => {
			run_and_respond(name, respond, || {
				ops::branch_create::run(repo, &branch_name, &revision)
			});
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
