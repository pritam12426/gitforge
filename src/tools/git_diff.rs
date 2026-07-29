use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_diff",
		"Show changes in the working tree (diff HEAD vs working directory)",
		json!({ "type": "object", "properties": {} }),
		Box::new(move |_| {
			let resp = call_actor(&repo, |respond| RepoCommand::GetDiff { respond })?;
			match resp {
				RepoResponse::Diff(text) => {
					if text.is_empty() {
						Ok(json!("no changes"))
					} else {
						Ok(json!(text))
					}
				}
				_ => Err(unexpected("git_diff")),
			}
		}),
	);
}
