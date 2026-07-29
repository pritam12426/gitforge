use std::path::Path;
use std::sync::mpsc;
use std::thread;

use crate::error::GitforgeError;

#[derive(Debug)]
pub enum RepoResponse {
	Health,
	Status(Vec<(String, String)>),
	Log(Vec<(String, String, String)>),
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
}
