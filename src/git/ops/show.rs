/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use std::io::Write;

use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(repo: &git2::Repository, revision: &str) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::show: revision='{}'", revision);
	let obj = repo.revparse_single(revision)?;
	let commit = obj.peel_to_commit()?;

	let hash = commit.id().to_string();
	let author = commit.author().name().unwrap_or("unknown").to_string();
	let email = commit.author().email().unwrap_or("").to_string();
	let time = commit.time().seconds();
	let message = commit.message().unwrap_or("").to_string();

	let tree = commit.tree()?;
	let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

	let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
	let mut diff_output: Vec<u8> = Vec::new();
	diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
		let origin = line.origin();
		let content = std::str::from_utf8(line.content()).unwrap_or("");
		let _ = write!(diff_output, "{}{}", origin, content);
		true
	})?;
	let diff_text = String::from_utf8(diff_output).unwrap();

	log_trace!(
		"ops::show: hash={} author={} diff={}b",
		hash,
		author,
		diff_text.len()
	);
	Ok(RepoResponse::ShowCommit(serde_json::json!({
		"hash": hash,
		"author": author,
		"email": email,
		"time": time,
		"message": message,
		"diff": diff_text,
	})))
}
