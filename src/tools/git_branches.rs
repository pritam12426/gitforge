use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::{ToolAnnotations, Router};
use crate::log_trace;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_branches");
	router.add_tool(
		"git_branches",
		"List branches",
		json!({ "type": "object", "properties": {} }),
		ToolAnnotations::read_only(),
		Box::new(move |_| {
			log_trace!("tools::git_branches: handling request");
			let resp = call_actor(&repo, |respond| RepoCommand::GetBranches { respond })?;
			match resp {
				RepoResponse::Branches(branches) => {
					log_trace!("tools::git_branches: {} branches", branches.len());
					let lines: Vec<String> = branches
						.iter()
						.map(|(name, is_head)| {
							if *is_head {
								format!("* {}", name)
							} else {
								format!("  {}", name)
							}
						})
						.collect();
					Ok(json!(lines.join("\n")))
				}
				_ => Err(unexpected("git_branches")),
			}
		}),
	);
}
