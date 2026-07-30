/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use std::io::Write;

use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::diff_staged: computing HEAD vs index diff");
	let head_tree = repo.head()?.peel_to_tree()?;
	let diff = repo.diff_tree_to_index(Some(&head_tree), None, None)?;

	let mut output: Vec<u8> = Vec::new();
	diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
		let origin = line.origin();
		let content = std::str::from_utf8(line.content()).unwrap_or("");
		let _ = write!(output, "{}{}", origin, content);
		true
	})?;

	let text = String::from_utf8(output).unwrap();
	log_trace!("ops::diff_staged: {} bytes diff output", text.len());
	Ok(RepoResponse::Diff(text))
}
