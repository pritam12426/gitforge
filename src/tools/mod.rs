use serde_json::json;

use crate::error::GitforgeError;
use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

pub fn register_all(router: &mut Router, repo: RepoHandle) {
	register_ping(router);
	register_git_status(router, repo.clone());
	register_git_log(router, repo);
}

fn register_ping(router: &mut Router) {
	router.add_tool(
		"ping",
		"Check if the server is alive",
		json!({ "type": "object", "properties": {} }),
		Box::new(|_| Ok(json!("pong"))),
	);
}

fn register_git_status(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_status",
		"Show the working tree status",
		json!({ "type": "object", "properties": {} }),
		Box::new(move |_| {
			let (tx, rx) = std::sync::mpsc::channel();
			repo.send(RepoCommand::GetStatus { respond: tx })?;
			let resp = rx.recv().map_err(|_| GitforgeError::ChannelClosed)??;
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
				_ => Err(GitforgeError::Internal("unexpected response".into())),
			}
		}),
	);
}

fn register_git_log(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_log",
		"Show commit history",
		json!({
			"type": "object",
			"properties": {
				"max_count": {
					"type": "integer",
					"description": "Maximum number of commits to show",
					"default": 10
				}
			}
		}),
		Box::new(move |args| {
			let max_count = args.get("max_count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

			let (tx, rx) = std::sync::mpsc::channel();
			repo.send(RepoCommand::GetLog {
				max_count,
				respond: tx,
			})?;
			let resp = rx.recv().map_err(|_| GitforgeError::ChannelClosed)??;
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
				_ => Err(GitforgeError::Internal("unexpected response".into())),
			}
		}),
	);
}
