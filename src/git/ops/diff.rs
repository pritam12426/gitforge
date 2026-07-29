use std::io::Write;

use crate::error::GitforgeError;
use crate::git::commands::RepoResponse;

pub fn run(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
	let head_tree = repo.head()?.peel_to_tree()?;
	let diff = repo.diff_tree_to_workdir(Some(&head_tree), None)?;

	let mut output: Vec<u8> = Vec::new();
	diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
		let origin = line.origin();
		let content = std::str::from_utf8(line.content()).unwrap_or("");
		let _ = write!(output, "{}{}", origin, content);
		true
	})?;

	let text = String::from_utf8_lossy(&output).to_string();
	Ok(RepoResponse::Diff(text))
}
