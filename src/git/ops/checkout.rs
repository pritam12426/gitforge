/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(repo: &git2::Repository, branch: &str) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::checkout: branch='{}'", branch);
	let obj = repo.revparse_single(branch)?;
	repo.checkout_tree(&obj, None)?;

	if branch.starts_with("refs/heads/") {
		repo.set_head(branch)?;
	} else {
		let full_ref = format!("refs/heads/{}", branch);
		if repo.find_reference(&full_ref).is_ok() {
			repo.set_head(&full_ref)?;
		} else {
			repo.set_head_detached(obj.id())?;
		}
	}
	log_trace!("ops::checkout: switched to '{}'", branch);
	Ok(RepoResponse::CheckoutOk)
}
