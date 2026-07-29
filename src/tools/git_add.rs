use serde_json::json;

use crate::error::GitforgeError;
use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::mcp::Router;

use super::{call_actor, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	router.add_tool(
		"git_add",
		"Stage file paths (pass [\".\"] to stage all)",
		json!({
			"type": "object",
			"properties": {
				"files": {
					"type": "array",
					"items": { "type": "string" },
					"description": "File paths to stage"
				}
			},
			"required": ["files"]
		}),
		Box::new(move |args| {
			let paths: Vec<String> = args
				.get("files")
				.and_then(|v| v.as_array())
				.map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
				.ok_or_else(|| GitforgeError::InvalidRequest("missing files".into()))?;

			let resp = call_actor(&repo, |respond| RepoCommand::StageFiles { paths, respond })?;
			match resp {
				RepoResponse::Staged => Ok(json!("staged")),
				_ => Err(unexpected("git_add")),
			}
		}),
	);
}
