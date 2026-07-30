use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_status");
	router.add_tool(
		"git_status",
		"Show the working tree status",
		json!({ "type": "object", "properties": {} }),
		ToolAnnotations::read_only(),
		Box::new(move |_| {
			log_trace!("tools::git_status: handling request");
			let resp = call_actor(&repo, |respond| RepoCommand::GetStatus { respond })?;
			match resp {
				RepoResponse::Status(entries) => {
					log_trace!("tools::git_status: {} entries", entries.len());
					if entries.is_empty() {
						Ok(json!("nothing to commit, working tree clean"))
					} else {
						let lines: Vec<String> = entries
							.iter()
							.map(|(path, status)| format!("{}  {}", status, path))
							.collect();
						Ok(json!(lines.join("\n")))
					}
				}
				_ => Err(unexpected("git_status")),
			}
		}),
	);
}
