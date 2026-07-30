use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(
	repo: &git2::Repository,
	branch_type: &str,
	contains: Option<&str>,
	not_contains: Option<&str>,
) -> Result<RepoResponse, GitforgeError> {
	log_trace!(
		"ops::branches: branch_type={} contains={:?} not_contains={:?}",
		branch_type,
		contains,
		not_contains
	);

	let filter = match branch_type {
		"remote" => Some(git2::BranchType::Remote),
		"all" => None,
		_ => Some(git2::BranchType::Local),
	};

	// Resolve the filter commits once (before the branch loop) so we
	// don't hit revparse in every iteration.
	let contains_commit = contains
		.map(|s| repo.revparse_single(s).and_then(|o| o.peel_to_commit()))
		.transpose()?;
	let not_contains_commit = not_contains
		.map(|s| repo.revparse_single(s).and_then(|o| o.peel_to_commit()))
		.transpose()?;

	let mut branches = Vec::new();
	for branch_result in repo.branches(filter)? {
		let (branch, _) = branch_result?;
		let name = branch.name()?.unwrap_or("").to_string();
		let is_head = branch.is_head();

		// Ancestry check via merge_base: if merge_base(A, B) == B then B
		// is an ancestor of A, meaning A "contains" B.
		if let (Some(target_commit), Some(cc)) =
			(branch.get().peel_to_commit().ok(), contains_commit.as_ref())
		{
			if repo.merge_base(target_commit.id(), cc.id()).ok() != Some(cc.id()) {
				continue;
			}
		}
		if let (Some(target_commit), Some(nc)) = (
			branch.get().peel_to_commit().ok(),
			not_contains_commit.as_ref(),
		) {
			if repo.merge_base(target_commit.id(), nc.id()).ok() == Some(nc.id()) {
				continue;
			}
		}

		branches.push((name, is_head));
	}
	log_trace!("ops::branches: {} branches after filtering", branches.len());
	Ok(RepoResponse::Branches(branches))
}
