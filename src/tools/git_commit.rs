use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_commit",
		"Stage all changes and create a new commit",
		json!({
			"type": "object",
			"properties": {
				"message": { "type": "string", "description": "Commit message" },
				"author_name": { "type": "string", "description": "Author name" },
				"author_email": { "type": "string", "description": "Author email" }
			},
			"required": ["message", "author_name", "author_email"]
		}),
		Box::new(move |args| {
			let message = required_str(&args, "message")?;
			let author_name = required_str(&args, "author_name")?;
			let author_email = required_str(&args, "author_email")?;

			let resp = call_actor(&repo, |respond| RepoCommand::CreateCommit {
				message,
				author_name,
				author_email,
				respond,
			})?;
			match resp {
				RepoResponse::CommitCreated(hash) => Ok(json!(format!("Created commit {}", hash))),
				_ => Err(unexpected("git_commit")),
			}
		}),
	);
}
