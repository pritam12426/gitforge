use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;

pub fn run(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
	let mut branches = Vec::new();
	for branch_result in repo.branches(None)? {
		let (branch, _) = branch_result?;
		let name = branch.name()?.unwrap_or("").to_string();
		let is_head = branch.is_head();
		branches.push((name, is_head));
	}
	Ok(RepoResponse::Branches(branches))
}
