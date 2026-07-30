use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_commit");
	router.add_tool(
		"git_commit",
		"Record staged changes as a new commit",
		json!({
			"type": "object",
			"properties": {
				"message": { "type": "string", "description": "Commit message" }
			},
			"required": ["message"]
		}),
		ToolAnnotations::mutable(),
		Box::new(move |args| {
			let message = required_str(&args, "message")?;
			let author_name = required_str(&args, "author_name")?;
			let author_email = required_str(&args, "author_email")?;
			log_trace!(
				"tools::git_commit: author='{}' msg_len={}",
				author_name,
				message.len()
			);

			let resp = call_actor(&repo, |respond| RepoCommand::CreateCommit {
				message,
				author_name,
				author_email,
				respond,
			})?;
			match resp {
				RepoResponse::CommitCreated(hash) => {
					log_trace!("tools::git_commit: created {}", hash);
					Ok(json!(format!("Created commit {}", hash)))
				}
				_ => Err(unexpected("git_commit")),
			}
		}),
	);
}
