use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(repo: &git2::Repository, paths: &[String]) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::stage: {} paths", paths.len());
	let mut index = repo.index()?;
	let patterns: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
	index.add_all(patterns.iter(), git2::IndexAddOption::DEFAULT, None)?;
	index.write()?;
	log_trace!("ops::stage: completed");
	Ok(RepoResponse::Staged)
}
