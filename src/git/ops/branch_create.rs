use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(
	repo: &git2::Repository,
	name: &str,
	revision: &str,
) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::branch_create: name='{}' revision='{}'", name, revision);
	let obj = repo.revparse_single(revision)?;
	let commit = obj.peel_to_commit()?;
	repo.branch(name, &commit, false)?;
	log_trace!("ops::branch_create: created '{}'", name);
	Ok(RepoResponse::BranchCreated(name.to_string()))
}
