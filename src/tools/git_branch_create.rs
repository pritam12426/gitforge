use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::{ToolAnnotations, Router};
use crate::log_trace;

use super::{call_actor, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_branch_create");
	router.add_tool(
		"git_branch_create",
		"Create a new branch",
		json!({
			"type": "object",
			"properties": {
				"name": { "type": "string", "description": "Branch name" },
				"revision": {
					"type": "string",
					"description": "Revision to branch from (default HEAD)",
					"default": "HEAD"
				}
			},
			"required": ["name"]
		}),
		ToolAnnotations::mutable(),
		Box::new(move |args| {
			let name = required_str(&args, "name")?;
			let revision = args.get("revision").and_then(|v| v.as_str()).unwrap_or("HEAD").to_string();
			log_trace!("tools::git_branch_create: name='{}' revision='{}'", name, revision);

			let resp = call_actor(&repo, |respond| RepoCommand::CreateBranch { name, revision, respond })?;
			match resp {
				RepoResponse::BranchCreated(name) => {
					log_trace!("tools::git_branch_create: created '{}'", name);
					Ok(json!(format!("Created branch '{}'", name)))
				}
				_ => Err(unexpected("git_branch_create")),
			}
		}),
	);
}
