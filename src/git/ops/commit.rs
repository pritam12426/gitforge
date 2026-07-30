use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(
	repo: &git2::Repository,
	message: &str,
	author_name: &str,
	author_email: &str,
) -> Result<RepoResponse, GitforgeError> {
	log_trace!(
		"ops::commit: author='{}' msg_len={}",
		author_name,
		message.len()
	);
	let mut index = repo.index()?;
	index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
	index.write()?;
	let tree_id = index.write_tree()?;
	let tree = repo.find_tree(tree_id)?;

	let parent_commit = repo.head()?.peel_to_commit()?;
	let signature = git2::Signature::now(author_name, author_email)?;
	let commit_id = repo.commit(
		Some("HEAD"),
		&signature,
		&signature,
		message,
		&tree,
		&[&parent_commit],
	)?;

	log_trace!("ops::commit: created {}", commit_id);
	Ok(RepoResponse::CommitCreated(commit_id.to_string()))
}
