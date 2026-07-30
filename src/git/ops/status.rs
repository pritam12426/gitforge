use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;
use crate::log_trace;

pub fn run(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
	log_trace!("ops::status: fetching working tree status");
	let statuses = repo.statuses(None)?;
	let entries: Vec<(String, String)> = statuses
		.iter()
		.filter_map(|entry| {
			let path = entry.path().ok()?.to_string();
			let status = format!("{:?}", entry.status());
			Some((path, status))
		})
		.collect();
	log_trace!("ops::status: {} entries", entries.len());
	Ok(RepoResponse::Status(entries))
}
