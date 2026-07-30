//! One file per tool (split out of what used to be a single 435-line
//! `tools/mod.rs`), all sharing the [`call_actor`] helper below so each
//! tool body only has to describe *its* request/response shape instead
//! of repeating "make a channel, send, recv, match" eleven times.

mod git_add;
mod git_branch_create;
mod git_branches;
mod git_checkout;
mod git_commit;
mod git_diff;
mod git_diff_staged;
mod git_diff_unstaged;
mod git_log;
mod git_merge;
mod git_show;
mod git_status;
mod ping;

use std::sync::mpsc;

use crate::error::GitforgeError;
use crate::git::{RepoCommand, RepoHandle, RepoResponse, recv_response};
use crate::log_trace;
use crate::mcp::Router;

pub fn register_all(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering all tool handlers");
	ping::register(router);
	git_status::register(router, repo.clone());
	git_log::register(router, repo.clone());
	git_branches::register(router, repo.clone());
	git_diff::register(router, repo.clone());
	git_diff_unstaged::register(router, repo.clone());
	git_diff_staged::register(router, repo.clone());
	git_show::register(router, repo.clone());
	git_commit::register(router, repo.clone());
	git_add::register(router, repo.clone());
	git_branch_create::register(router, repo.clone());
	git_checkout::register(router, repo.clone());
	git_merge::register(router, repo);
	log_trace!("tools: all tool handlers registered");
}

/// Sends one command to the repo actor and waits for its response.
/// `build` receives the response channel and constructs the
/// `RepoCommand` variant — this is the one bit of boilerplate every tool
/// still needs, since each `RepoCommand` variant carries different
/// fields.
fn call_actor(
	repo: &RepoHandle,
	build: impl FnOnce(mpsc::Sender<Result<RepoResponse, GitforgeError>>) -> RepoCommand,
) -> Result<RepoResponse, GitforgeError> {
	let (tx, rx) = mpsc::channel();
	repo.send(build(tx))?;
	recv_response(rx)
}

/// Every tool ultimately expects one specific `RepoResponse` variant
/// back; getting anything else means an actor/dispatch bug rather than
/// a user-facing error, so it's reported as `Internal` rather than
/// something like `OperationFailed`.
fn unexpected(op: &str) -> GitforgeError {
	GitforgeError::Internal(format!("unexpected actor response for {op}"))
}

/// Pulls a required string field out of a tool's `arguments` object,
/// returning a well-formed `InvalidRequest` (not a panic or a generic
/// `Internal`) when it's missing or the wrong type.
fn required_str(args: &serde_json::Value, field: &str) -> Result<String, GitforgeError> {
	args.get(field)
		.and_then(|v| v.as_str())
		.map(|s| s.to_string())
		.ok_or_else(|| GitforgeError::InvalidRequest(format!("missing {field}")))
}

/// Defense-in-depth: reject values starting with `-`.
///
/// Git CLI commands interpret positional arguments that start with `-`
/// as flags (e.g. `git checkout --help` prints help instead of switching
/// branches), so a malicious or accidentally-misnamed branch / revision /
/// timestamp could trigger unexpected behaviour.  This check rejects
/// such values before they reach the git layer, even if git2 itself
/// would handle them correctly — defence in depth.
///
/// Applied in `git_show`, `git_log`, `git_branch_create`, `git_checkout`,
/// `git_branches`, and `git_diff` tool handlers.
fn reject_flag(val: &str, field: &str) -> Result<(), GitforgeError> {
	if val.starts_with('-') {
		return Err(GitforgeError::InvalidRequest(format!(
			"{field} cannot start with '-'"
		)));
	}
	Ok(())
}
