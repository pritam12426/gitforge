use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_branches",
		"List branches",
		json!({ "type": "object", "properties": {} }),
		Box::new(move |_| {
			let resp = call_actor(&repo, |respond| RepoCommand::GetBranches { respond })?;
			match resp {
				RepoResponse::Branches(branches) => {
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
