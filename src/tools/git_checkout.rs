use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, reject_flag, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_checkout");
	router.add_tool(
		"git_checkout",
		"Switch to a different branch",
		json!({
			"type": "object",
			"properties": {
				"branch_name": { "type": "string", "description": "Branch to switch to" }
			},
			"required": ["branch_name"]
		}),
		ToolAnnotations::mutable(),
		Box::new(move |args| {
			let branch = required_str(&args, "branch")?;
			reject_flag(&branch, "branch")?;
			log_trace!("tools::git_checkout: branch='{}'", branch);

			let resp = call_actor(&repo, |respond| RepoCommand::Checkout { branch, respond })?;
			match resp {
				RepoResponse::CheckoutOk => {
					log_trace!("tools::git_checkout: switched branch");
					Ok(json!("switched branch"))
				}
				_ => Err(unexpected("git_checkout")),
			}
		}),
	);
}
