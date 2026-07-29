use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crate::error::GitforgeError;
use crate::{log_error, log_info};

use super::actor;
use super::commands::{RepoCommand, RepoResponse};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct RepoHandle {
	sender: mpsc::Sender<RepoCommand>,
}

impl RepoHandle {
	/// Opens the repository and spawns the actor thread that owns it.
	/// `git2::Repository` is `!Sync`, so all access is funneled through
	/// this one thread and an mpsc channel — cloning `RepoHandle` just
	/// clones the `Sender`, it never touches the repo directly.
	pub fn spawn(path: &Path) -> Result<Self, GitforgeError> {
		let repo = git2::Repository::open(path).map_err(|e| {
			GitforgeError::Internal(format!("failed to open repo at '{}': {}", path.display(), e))
		})?;
		log_info!("git actor: opened repo at {}", path.display());

		let (sender, receiver) = mpsc::channel::<RepoCommand>();
		std::thread::Builder::new()
			.name("gitforge-repo-actor".into())
			.spawn(move || actor::run(repo, receiver))
			.map_err(GitforgeError::Io)?;

		Ok(RepoHandle { sender })
	}

	pub fn send(&self, cmd: RepoCommand) -> Result<(), GitforgeError> {
		self.sender.send(cmd).map_err(|_| {
			log_error!("git actor: send failed — actor thread is gone");
			GitforgeError::Actor("repo actor thread has shut down".into())
		})
	}
}

/// Receive a response from the actor, with a bounded wait so the server
/// never hangs forever on a stuck git2 call.
pub fn recv_response(
	rx: mpsc::Receiver<Result<RepoResponse, GitforgeError>>,
) -> Result<RepoResponse, GitforgeError> {
	match rx.recv_timeout(REQUEST_TIMEOUT) {
		Ok(result) => result,
		Err(mpsc::RecvTimeoutError::Timeout) => {
			Err(GitforgeError::Actor("request to repo actor timed out after 30s".into()))
		}
		Err(mpsc::RecvTimeoutError::Disconnected) => {
			Err(GitforgeError::Actor("repo actor thread disconnected".into()))
		}
	}
}
