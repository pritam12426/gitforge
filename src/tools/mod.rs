use serde_json::json;

use crate::error::GitforgeError;
use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

pub fn register_all(router: &mut Router, repo: RepoHandle) {
	register_ping(router);
	register_git_status(router, repo.clone());
	register_git_log(router, repo.clone());
	register_git_branches(router, repo.clone());
	register_git_diff(router, repo.clone());
	register_git_show(router, repo.clone());
	register_git_commit(router, repo);
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

fn register_git_branches(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_branches",
		"List branches",
		json!({ "type": "object", "properties": {} }),
		Box::new(move |_| {
			let (tx, rx) = std::sync::mpsc::channel();
			repo.send(RepoCommand::GetBranches { respond: tx })?;
			let resp = rx.recv().map_err(|_| GitforgeError::ChannelClosed)??;
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
				_ => Err(GitforgeError::Internal("unexpected response".into())),
			}
		}),
	);
}

fn register_git_diff(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_diff",
		"Show changes in the working tree (diff HEAD vs working directory)",
		json!({ "type": "object", "properties": {} }),
		Box::new(move |_| {
			let (tx, rx) = std::sync::mpsc::channel();
			repo.send(RepoCommand::GetDiff { respond: tx })?;
			let resp = rx.recv().map_err(|_| GitforgeError::ChannelClosed)??;
			match resp {
				RepoResponse::Diff(text) => {
					if text.is_empty() {
						Ok(json!("no changes"))
					} else {
						Ok(json!(text))
					}
				}
				_ => Err(GitforgeError::Internal("unexpected response".into())),
			}
		}),
	);
}

fn register_git_show(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_show",
		"Show details of a commit (hash, author, message, diff)",
		json!({
			"type": "object",
			"properties": {
				"revision": {
					"type": "string",
					"description": "Revision (commit hash, branch name, or ref)",
					"default": "HEAD"
				}
			}
		}),
		Box::new(move |args| {
			let revision = args
				.get("revision")
				.and_then(|v| v.as_str())
				.unwrap_or("HEAD")
				.to_string();

			let (tx, rx) = std::sync::mpsc::channel();
			repo.send(RepoCommand::ShowCommit {
				revision,
				respond: tx,
			})?;
			let resp = rx.recv().map_err(|_| GitforgeError::ChannelClosed)??;
			match resp {
				RepoResponse::ShowCommit(info) => {
					let hash = info["hash"].as_str().unwrap_or("");
					let author = info["author"].as_str().unwrap_or("");
					let email = info["email"].as_str().unwrap_or("");
					let time = info["time"].as_i64().unwrap_or(0);
					let message = info["message"].as_str().unwrap_or("");
					let diff = info["diff"].as_str().unwrap_or("");

					// Format time
					#[cfg(feature = "show_time_stamp")]
					let datetime = {
						let naive = std::time::UNIX_EPOCH
							+ std::time::Duration::from_secs(time as u64);
						let datetime: chrono::DateTime<chrono::Local> = naive.into();
						format!("{}", datetime.format("%a %b %e %H:%M:%S %Y"))
					};
					#[cfg(not(feature = "show_time_stamp"))]
					let datetime = time.to_string();

					let mut output = String::new();
					output.push_str(&format!("commit {}\n", hash));
					output.push_str(&format!("Author: {} <{}>\n", author, email));
					output.push_str(&format!("Date:   {}\n", datetime));
					output.push('\n');
					output.push_str(message);
					output.push('\n');
					if !diff.is_empty() {
						output.push_str(&diff);
					}

					Ok(json!(output))
				}
				_ => Err(GitforgeError::Internal("unexpected response".into())),
			}
		}),
	);
}

fn register_git_commit(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_commit",
		"Stage all changes and create a new commit",
		json!({
			"type": "object",
			"properties": {
				"message": {
					"type": "string",
					"description": "Commit message"
				},
				"author_name": {
					"type": "string",
					"description": "Author name"
				},
				"author_email": {
					"type": "string",
					"description": "Author email"
				}
			},
			"required": ["message", "author_name", "author_email"]
		}),
		Box::new(move |args| {
			let message = args
				.get("message")
				.and_then(|v| v.as_str())
				.ok_or_else(|| GitforgeError::Internal("missing message".into()))?
				.to_string();
			let author_name = args
				.get("author_name")
				.and_then(|v| v.as_str())
				.ok_or_else(|| GitforgeError::Internal("missing author_name".into()))?
				.to_string();
			let author_email = args
				.get("author_email")
				.and_then(|v| v.as_str())
				.ok_or_else(|| GitforgeError::Internal("missing author_email".into()))?
				.to_string();

			let (tx, rx) = std::sync::mpsc::channel();
			repo.send(RepoCommand::CreateCommit {
				message,
				author_name,
				author_email,
				respond: tx,
			})?;
			let resp = rx.recv().map_err(|_| GitforgeError::ChannelClosed)??;
			match resp {
				RepoResponse::CommitCreated(hash) => {
					Ok(json!(format!("Created commit {}", hash)))
				}
				_ => Err(GitforgeError::Internal("unexpected response".into())),
			}
		}),
	);
}
