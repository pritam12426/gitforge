/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use crate::error::GitforgeError;
use crate::git::{RepoCommand, RepoHandle, RepoResponse, recv_response};
use crate::{log_error, log_trace};

#[derive(Clone)]
pub struct Resource {
	pub uri: String,
	pub name: String,
	pub description: String,
	pub mime_type: String,
}

pub fn builtin_resources() -> Vec<Resource> {
	vec![
		Resource {
			uri: "git://HEAD".into(),
			name: "HEAD commit".into(),
			description: "Current HEAD commit details (hash, author, message)".into(),
			mime_type: "text/plain".into(),
		},
		Resource {
			uri: "git://status".into(),
			name: "Working tree status".into(),
			description: "Current working tree status".into(),
			mime_type: "text/plain".into(),
		},
	]
}

pub fn fetch_content(repo: &RepoHandle, uri: &str) -> Result<String, GitforgeError> {
	log_trace!("resources: fetching '{}'", uri);
	let result = match uri {
		"git://HEAD" => fetch_head_info(repo),
		"git://status" => fetch_status(repo),
		other => Err(GitforgeError::NotFound(format!(
			"unknown resource: {}",
			other
		))),
	};
	match &result {
		Ok(text) => log_trace!("resources: '{}' returned {} bytes", uri, text.len()),
		Err(e) => log_error!("resources: '{}' failed: {}", uri, e),
	}
	result
}

fn fetch_head_info(repo: &RepoHandle) -> Result<String, GitforgeError> {
	let (tx, rx) = std::sync::mpsc::channel();
	repo.send(RepoCommand::ShowCommit {
		revision: "HEAD".into(),
		respond: tx,
	})?;
	let resp = recv_response(rx)?;
	match resp {
		RepoResponse::ShowCommit(info) => {
			let hash = info["hash"].as_str().unwrap_or("");
			let author = info["author"].as_str().unwrap_or("");
			let email = info["email"].as_str().unwrap_or("");
			let time = info["time"].as_i64().unwrap_or(0);
			let message = info["message"].as_str().unwrap_or("");

			#[cfg(feature = "show_time_stamp")]
			let datetime = {
				let naive = std::time::UNIX_EPOCH + std::time::Duration::from_secs(time as u64);
				let datetime: chrono::DateTime<chrono::Local> = naive.into();
				format!("{}", datetime.format("%a %b %e %H:%M:%S %Y"))
			};
			#[cfg(not(feature = "show_time_stamp"))]
			let datetime = time.to_string();

			let mut out = String::new();
			out.push_str(&format!("commit {}\n", hash));
			out.push_str(&format!("Author: {} <{}>\n", author, email));
			out.push_str(&format!("Date:   {}\n", datetime));
			out.push('\n');
			out.push_str(message);
			out.push('\n');
			Ok(out)
		}
		_ => Err(GitforgeError::Internal(
			"unexpected actor response for ShowCommit".into(),
		)),
	}
}

fn fetch_status(repo: &RepoHandle) -> Result<String, GitforgeError> {
	let (tx, rx) = std::sync::mpsc::channel();
	repo.send(RepoCommand::GetStatus { respond: tx })?;
	let resp = recv_response(rx)?;
	match resp {
		RepoResponse::Status(entries) => {
			if entries.is_empty() {
				Ok("nothing to commit, working tree clean".into())
			} else {
				let lines: Vec<String> = entries
					.iter()
					.map(|(path, status)| format!("{}  {}", status, path))
					.collect();
				Ok(lines.join("\n"))
			}
		}
		_ => Err(GitforgeError::Internal(
			"unexpected actor response for GetStatus".into(),
		)),
	}
}
