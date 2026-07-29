use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_checkout",
		"Switch to a branch",
		json!({
			"type": "object",
			"properties": {
				"branch": { "type": "string", "description": "Branch name to switch to" }
			},
			"required": ["branch"]
		}),
		Box::new(move |args| {
			let branch = required_str(&args, "branch")?;

			let resp = call_actor(&repo, |respond| RepoCommand::Checkout { branch, respond })?;
			match resp {
				RepoResponse::CheckoutOk => Ok(json!("switched branch")),
				_ => Err(unexpected("git_checkout")),
			}
		}),
	);
}
