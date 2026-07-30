/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use std::path::Path;

use serde_json::json;

use crate::error::GitforgeError;
use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, unexpected};

/// Rejects paths that resolve outside the allowed repository boundary.
///
/// Uses `canonicalize()` to resolve symlinks, `..`, and relative paths
/// (e.g. `../../etc/passwd`) to their real filesystem location, then
/// checks that the result lives under `allowed_repo`.  This prevents
/// path-traversal attacks where a crafted file argument would stage
/// files outside the repository.
fn path_is_allowed(path: &str, allowed_repo: &Path) -> Result<(), GitforgeError> {
	let p = Path::new(path);
	let canonical = p
		.canonicalize()
		.map_err(|_| GitforgeError::Forbidden(format!("cannot resolve path '{}'", path)))?;
	if !canonical.starts_with(allowed_repo) {
		return Err(GitforgeError::Forbidden(format!(
			"path '{}' resolves outside allowed repository '{}'",
			path,
			allowed_repo.display()
		)));
	}
	Ok(())
}

pub fn register(router: &mut Router, repo: RepoHandle) {
	let allowed = router.allowed_repo().map(|p| p.to_path_buf());

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
		ToolAnnotations {
			read_only_hint: false,
			destructive_hint: false,
			idempotent_hint: true,
			open_world_hint: false,
		},
		Box::new(move |args| {
			let paths: Vec<String> = args
				.get("files")
				.and_then(|v| v.as_array())
				.map(|arr| {
					arr.iter()
						.filter_map(|v| v.as_str().map(|s| s.to_string()))
						.collect()
				})
				.ok_or_else(|| GitforgeError::InvalidRequest("missing files".into()))?;
			log_trace!("tools::git_add: {} paths to stage", paths.len());

			if let Some(ref allowed) = allowed {
				for p in &paths {
					path_is_allowed(p, allowed)?;
				}
				log_trace!("tools::git_add: path validation passed");
			}

			let resp = call_actor(&repo, |respond| RepoCommand::StageFiles { paths, respond })?;
			match resp {
				RepoResponse::Staged => {
					log_trace!("tools::git_add: staged successfully");
					Ok(json!("staged"))
				}
				_ => Err(unexpected("git_add")),
			}
		}),
	);
}
