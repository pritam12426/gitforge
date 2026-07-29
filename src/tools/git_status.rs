use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_status",
		"Show the working tree status",
		json!({ "type": "object", "properties": {} }),
		Box::new(move |_| {
			let resp = call_actor(&repo, |respond| RepoCommand::GetStatus { respond })?;
			match resp {
				RepoResponse::Status(entries) => {
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
