use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use crate::error::GitforgeError;

#[derive(Debug)]
pub enum RepoResponse {
	Health,
	Status(Vec<(String, String)>),
	Log(Vec<(String, String, String)>),
	Branches(Vec<(String, bool)>),
	Diff(String),
	ShowCommit(serde_json::Value),
	CommitCreated(String),
}

pub enum RepoCommand {
	CheckHealth {
		respond: mpsc::Sender<Result<RepoResponse, GitforgeError>>,
	},
	GetStatus {
		respond: mpsc::Sender<Result<RepoResponse, GitforgeError>>,
	},
	GetLog {
		max_count: usize,
		respond: mpsc::Sender<Result<RepoResponse, GitforgeError>>,
	},
	GetBranches {
		respond: mpsc::Sender<Result<RepoResponse, GitforgeError>>,
	},
	GetDiff {
		respond: mpsc::Sender<Result<RepoResponse, GitforgeError>>,
	},
	ShowCommit {
		revision: String,
		respond: mpsc::Sender<Result<RepoResponse, GitforgeError>>,
	},
	CreateCommit {
		message: String,
		author_name: String,
		author_email: String,
		respond: mpsc::Sender<Result<RepoResponse, GitforgeError>>,
	},
}

#[derive(Clone)]
pub struct RepoHandle {
	sender: mpsc::Sender<RepoCommand>,
}

impl RepoHandle {
	pub fn spawn(path: &Path) -> Result<Self, GitforgeError> {
		let repo = git2::Repository::open(path).map_err(|e| {
			GitforgeError::Internal(format!(
				"failed to open repo at '{}': {}",
				path.display(),
				e
			))
		})?;
		let (sender, receiver) = mpsc::channel::<RepoCommand>();

		thread::spawn(move || {
			while let Ok(cmd) = receiver.recv() {
				match cmd {
					RepoCommand::CheckHealth { respond } => {
						let _ = respond.send(Ok(RepoResponse::Health));
					}
					RepoCommand::GetStatus { respond } => {
						let _ = respond.send(Self::do_status(&repo));
					}
					RepoCommand::GetLog { max_count, respond } => {
						let _ = respond.send(Self::do_log(&repo, max_count));
					}
					RepoCommand::GetBranches { respond } => {
						let _ = respond.send(Self::do_branches(&repo));
					}
					RepoCommand::GetDiff { respond } => {
						let _ = respond.send(Self::do_diff(&repo));
					}
					RepoCommand::ShowCommit { revision, respond } => {
						let _ = respond.send(Self::do_show(&repo, &revision));
					}
					RepoCommand::CreateCommit {
						message,
						author_name,
						author_email,
						respond,
					} => {
						let _ =
							respond.send(Self::do_commit(&repo, &message, &author_name, &author_email));
					}
				}
			}
		});

		Ok(RepoHandle { sender })
	}

	pub fn send(&self, cmd: RepoCommand) -> Result<(), GitforgeError> {
		self.sender
			.send(cmd)
			.map_err(|_| GitforgeError::ChannelClosed)
	}

	fn do_status(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
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

	fn do_log(repo: &git2::Repository, max_count: usize) -> Result<RepoResponse, GitforgeError> {
		let mut revwalk = repo.revwalk()?;
		revwalk.push_head()?;
		revwalk.set_sorting(git2::Sort::TIME)?;

		let mut entries = Vec::new();
		for (i, oid) in revwalk.enumerate() {
			if i >= max_count {
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

	fn do_branches(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
		let mut branches = Vec::new();
		for branch_result in repo.branches(None)? {
			let (branch, _) = branch_result?;
			let name = branch.name()?.unwrap_or("").to_string();
			let is_head = branch.is_head();
			branches.push((name, is_head));
		}
		Ok(RepoResponse::Branches(branches))
	}

	fn do_diff(repo: &git2::Repository) -> Result<RepoResponse, GitforgeError> {
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

	fn do_show(repo: &git2::Repository, revision: &str) -> Result<RepoResponse, GitforgeError> {
		let obj = repo.revparse_single(revision)?;
		let commit = obj.peel_to_commit()?;

		let hash = commit.id().to_string();
		let author = commit.author().name().unwrap_or("unknown").to_string();
		let email = commit.author().email().unwrap_or("").to_string();
		let time = commit.time().seconds();
		let message = commit.message().unwrap_or("").to_string();

		// Get diff against parent
		let tree = commit.tree()?;
		let parent_tree = commit
			.parent(0)
			.ok()
			.and_then(|p| p.tree().ok());

		let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
		let mut diff_output: Vec<u8> = Vec::new();
		diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
			let origin = line.origin();
			let content = std::str::from_utf8(line.content()).unwrap_or("");
			let _ = write!(diff_output, "{}{}", origin, content);
			true
		})?;
		let diff_text = String::from_utf8_lossy(&diff_output).to_string();

		Ok(RepoResponse::ShowCommit(serde_json::json!({
			"hash": hash,
			"author": author,
			"email": email,
			"time": time,
			"message": message,
			"diff": diff_text,
		})))
	}

	fn do_commit(
		repo: &git2::Repository,
		message: &str,
		author_name: &str,
		author_email: &str,
	) -> Result<RepoResponse, GitforgeError> {
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

		Ok(RepoResponse::CommitCreated(commit_id.to_string()))
	}
}
