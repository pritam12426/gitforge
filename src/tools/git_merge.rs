use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_merge",
		"Merge a branch into the current HEAD",
		json!({
			"type": "object",
			"properties": {
				"branch": { "type": "string", "description": "Branch to merge" }
			},
			"required": ["branch"]
		}),
		Box::new(move |args| {
			let branch = required_str(&args, "branch")?;

			let resp = call_actor(&repo, |respond| RepoCommand::Merge { branch, respond })?;
			match resp {
				RepoResponse::MergeOk(msg) => Ok(json!(msg)),
				_ => Err(unexpected("git_merge")),
			}
		}),
	);
}
