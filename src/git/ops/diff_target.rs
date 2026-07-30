/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use std::io::Write;

use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

/// Compares `target`'s tree against HEAD's tree, producing a standard
/// git patch.  Uses `diff_tree_to_tree` (tree-vs-tree) rather than
/// `diff_tree_to_workdir` because the latter skips files that don't exist
/// in the target tree (i.e. newly added files on the current branch),
/// which produces an incomplete diff compared to what `git diff <target>`
/// would normally show.
pub fn run(repo: &git2::Repository, target: &str) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::diff_target: target='{}'", target);
	let target_obj = repo.revparse_single(target)?;
	let target_commit = target_obj.peel_to_commit()?;
	let target_tree = target_commit.tree()?;
	let head_tree = repo.head()?.peel_to_tree()?;
	let diff = repo.diff_tree_to_tree(Some(&target_tree), Some(&head_tree), None)?;

	let mut output: Vec<u8> = Vec::new();
	diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
		let origin = line.origin();
		let content = std::str::from_utf8(line.content()).unwrap_or("");
		let _ = write!(output, "{}{}", origin, content);
		true
	})?;

	let text = String::from_utf8(output).unwrap();
	log_trace!("ops::diff_target: {} bytes diff output", text.len());
	Ok(RepoResponse::Diff(text))
}
