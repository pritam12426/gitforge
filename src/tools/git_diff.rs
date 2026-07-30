use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::{ToolAnnotations, Router};
use crate::log_trace;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_diff");
	router.add_tool(
		"git_diff",
		"Show changes in the working tree (diff HEAD vs working directory)",
		json!({ "type": "object", "properties": {} }),
		ToolAnnotations::read_only(),
		Box::new(move |_| {
			log_trace!("tools::git_diff: handling request");
			let resp = call_actor(&repo, |respond| RepoCommand::GetDiff { respond })?;
			match resp {
				RepoResponse::Diff(text) => {
					log_trace!("tools::git_diff: {} bytes diff", text.len());
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
