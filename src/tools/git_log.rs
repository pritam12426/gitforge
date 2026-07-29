use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_log",
		"Show commit history",
		json!({
			"type": "object",
			"properties": {
				"offset": { "type": "integer", "description": "Number of commits to skip", "default": 0 },
				"limit": { "type": "integer", "description": "Maximum number of commits to show", "default": 10 }
			}
		}),
		Box::new(move |args| {
			let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
			let max_count = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

			let resp =
				call_actor(&repo, |respond| RepoCommand::GetLog { offset, max_count, respond })?;
			match resp {
				RepoResponse::Log(entries) => {
					let lines: Vec<String> = entries
						.iter()
						.map(|(hash, author, subject)| {
							let short_hash: String = hash.chars().take(7).collect();
							format!("{}  {}  {}", short_hash, author, subject)
						})
						.collect();
					Ok(json!(lines.join("\n")))
				}
				_ => Err(unexpected("git_log")),
			}
		}),
	);
}
