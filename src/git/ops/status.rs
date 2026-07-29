use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;

pub fn run(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
	let statuses = repo.statuses(None)?;
	let entries = statuses
		.iter()
		.filter_map(|entry| {
			let path = entry.path().ok()?.to_string();
			let status = format!("{:?}", entry.status());
			Some((path, status))
		})
		.collect();
	Ok(RepoResponse::Status(entries))
}
