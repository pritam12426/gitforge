use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;

pub fn run(
	repo: &git2::Repository,
	offset: usize,
	max_count: usize,
) -> Result<RepoResponse, GitforgeError> {
	let mut revwalk = repo.revwalk()?;
	revwalk.push_head()?;
	revwalk.set_sorting(git2::Sort::TIME)?;

	let mut entries = Vec::new();
	for (i, oid) in revwalk.enumerate() {
		if i < offset {
			continue;
		}
		if entries.len() >= max_count {
			break;
		}
		let oid = oid?;
		let commit = repo.find_commit(oid)?;
		let hash = oid.to_string();
		let author = commit.author().name().unwrap_or("unknown").to_string();
		let msg = commit.message().unwrap_or("").to_string();
		let subject = msg.lines().next().unwrap_or("").to_string();
		entries.push((hash, author, subject));
	}

	Ok(RepoResponse::Log(entries))
}
