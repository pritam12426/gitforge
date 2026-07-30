use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::{ToolAnnotations, Router};
use crate::log_trace;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_log");
	router.add_tool(
		"git_log",
		"Show commit logs (supports optional date filtering)",
		json!({
			"type": "object",
			"properties": {
				"max_count": { "type": "integer", "description": "Max commits (default 10)" },
				"start_timestamp": { "type": "string", "description": "Filter commits after this date (ISO 8601, relative, or absolute)" },
				"end_timestamp": { "type": "string", "description": "Filter commits before this date (ISO 8601, relative, or absolute)" }
			}
		}),
		ToolAnnotations::read_only(),
		Box::new(move |args| {
			let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
			let max_count = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
			log_trace!("tools::git_log: offset={} max_count={}", offset, max_count);

			let resp =
				call_actor(&repo, |respond| RepoCommand::GetLog { offset, max_count, respond })?;
			match resp {
				RepoResponse::Log(entries) => {
					log_trace!("tools::git_log: got {} entries", entries.len());
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
