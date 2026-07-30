/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(
	repo: &git2::Repository,
	offset: usize,
	max_count: usize,
) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::log: offset={} max_count={}", offset, max_count);
	let mut revwalk = repo.revwalk()?;
	revwalk.push_head()?;
	revwalk.set_sorting(git2::Sort::TIME)?;

	if offset > 0 {
		revwalk.nth(offset - 1);
	}

	let mut entries = Vec::new();
	for oid in revwalk {
		if entries.len() >= max_count {
			break;
		}
		let oid = oid?;
		let commit = repo.find_commit(oid)?;
		let hash = oid.to_string();
		let author = commit.author().name().unwrap_or("unknown").to_string();
		let msg = commit.message().unwrap_or("").to_string();
		let subject = msg.lines().next().unwrap_or("").to_string();
		entries.push((hash, author, subject));
	}

	log_trace!("ops::log: returning {} entries", entries.len());
	Ok(RepoResponse::Log(entries))
}
