use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;

pub fn run(
	repo: &git2::Repository,
	name: &str,
	revision: &str,
) -> Result<RepoResponse, GitforgeError> {
	let obj = repo.revparse_single(revision)?;
	let commit = obj.peel_to_commit()?;
	repo.branch(name, &commit, false)?;
	Ok(RepoResponse::BranchCreated(name.to_string()))
}
