use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_merge");
	router.add_tool(
		"git_merge",
		"Merge another branch into the current branch",
		json!({
			"type": "object",
			"properties": {
				"branch": { "type": "string", "description": "Branch to merge" }
			},
			"required": ["branch"]
		}),
		ToolAnnotations {
			read_only_hint: false,
			destructive_hint: true,
			idempotent_hint: false,
			open_world_hint: false,
		},
		Box::new(move |args| {
			let branch = required_str(&args, "branch")?;
			log_trace!("tools::git_merge: target='{}'", branch);

			let resp = call_actor(&repo, |respond| RepoCommand::Merge { branch, respond })?;
			match resp {
				RepoResponse::MergeOk(msg) => {
					log_trace!("tools::git_merge: result='{}'", msg);
					Ok(json!(msg))
				}
				_ => Err(unexpected("git_merge")),
			}
		}),
	);
}
