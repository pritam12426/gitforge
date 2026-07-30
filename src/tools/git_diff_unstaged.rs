use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_diff_unstaged");
	router.add_tool(
		"git_diff_unstaged",
		"Show changes in the working directory that are not yet staged",
		json!({ "type": "object", "properties": {} }),
		ToolAnnotations::read_only(),
		Box::new(move |_| {
			log_trace!("tools::git_diff_unstaged: handling request");
			let resp = call_actor(&repo, |respond| RepoCommand::GetDiffUnstaged { respond })?;
			match resp {
				RepoResponse::Diff(text) => {
					log_trace!("tools::git_diff_unstaged: {} bytes diff", text.len());
					if text.is_empty() {
						Ok(json!("no unstaged changes"))
					} else {
						Ok(json!(text))
					}
				}
				_ => Err(unexpected("git_diff_unstaged")),
			}
		}),
	);
}
