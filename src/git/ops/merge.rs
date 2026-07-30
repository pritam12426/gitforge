/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(repo: &git2::Repository, branch: &str) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::merge: target='{}'", branch);
	let head = repo.head()?.peel_to_commit()?;
	let obj = repo.revparse_single(branch)?;
	let other = obj.peel_to_commit()?;

	let merge_base = repo.merge_base(head.id(), other.id())?;
	if merge_base == other.id() {
		log_trace!("ops::merge: already up-to-date");
		return Ok(RepoResponse::MergeOk("already up-to-date".into()));
	}
	if merge_base == head.id() {
		log_trace!("ops::merge: fast-forward to '{}'", branch);
		repo.checkout_tree(&obj, None)?;
		let refname = format!("refs/heads/{}", branch);
		repo.set_head(&refname)?;
		return Ok(RepoResponse::MergeOk(format!("fast-forward to {}", branch)));
	}

	// Three-way merge.
	log_trace!("ops::merge: three-way merge with '{}'", branch);
	let head_tree = head.tree()?;
	let other_tree = other.tree()?;
	let base_commit = repo.find_commit(merge_base)?;
	let base_tree = base_commit.tree()?;

	let mut index = repo.merge_trees(&base_tree, &head_tree, &other_tree, None)?;
	if index.has_conflicts() {
		log_trace!("ops::merge: conflicts with '{}'", branch);
		return Err(GitforgeError::OperationFailed(format!(
			"merge conflicts with '{}'",
			branch
		)));
	}

	let tree_id = index.write_tree_to(repo)?;
	let tree = repo.find_tree(tree_id)?;
	let signature = git2::Signature::now("gitforge", "gitforge@mcp")?;
	repo.commit(
		Some("HEAD"),
		&signature,
		&signature,
		&format!("Merge branch '{}'", branch),
		&tree,
		&[&head, &other],
	)?;

	log_trace!("ops::merge: completed merge of '{}'", branch);
	Ok(RepoResponse::MergeOk(format!("merged '{}'", branch)))
}
