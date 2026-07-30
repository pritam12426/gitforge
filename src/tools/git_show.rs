use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, reject_flag, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_show");
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
		ToolAnnotations::read_only(),
		Box::new(move |args| {
			let revision = args
				.get("revision")
				.and_then(|v| v.as_str())
				.unwrap_or("HEAD")
				.to_string();
			if revision != "HEAD" {
				reject_flag(&revision, "revision")?;
			}
			log_trace!("tools::git_show: revision='{}'", revision);

			let resp = call_actor(&repo, |respond| RepoCommand::ShowCommit {
				revision,
				respond,
			})?;
			match resp {
				RepoResponse::ShowCommit(info) => {
					let hash = info["hash"].as_str().unwrap_or("");
					let author = info["author"].as_str().unwrap_or("");
					let email = info["email"].as_str().unwrap_or("");
					let time = info["time"].as_i64().unwrap_or(0);
					let message = info["message"].as_str().unwrap_or("");
					let diff = info["diff"].as_str().unwrap_or("");

					#[cfg(feature = "show_time_stamp")]
					let datetime = {
						let naive =
							std::time::UNIX_EPOCH + std::time::Duration::from_secs(time as u64);
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
						output.push_str(diff);
					}

					log_trace!("tools::git_show: hash={} author={}", hash, author);
					Ok(json!(output))
				}
				_ => Err(unexpected("git_show")),
			}
		}),
	);
}
