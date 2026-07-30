/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, reject_flag, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_branches");
	router.add_tool(
		"git_branches",
		"List Git branches",
		json!({
			"type": "object",
			"properties": {
				"branch_type": {
					"type": "string",
					"description": "Branch type: 'local', 'remote', or 'all'",
					"default": "local"
				},
				"contains": {
					"type": "string",
					"description": "Commit SHA that branches must contain"
				},
				"not_contains": {
					"type": "string",
					"description": "Commit SHA that branches must not contain"
				}
			}
		}),
		ToolAnnotations::read_only(),
		Box::new(move |args| {
			let branch_type = args
				.get("branch_type")
				.and_then(|v| v.as_str())
				.unwrap_or("local")
				.to_string();
			let contains = args.get("contains").and_then(|v| v.as_str());
			let not_contains = args.get("not_contains").and_then(|v| v.as_str());

			if let Some(val) = contains {
				reject_flag(val, "contains")?;
			}
			if let Some(val) = not_contains {
				reject_flag(val, "not_contains")?;
			}

			log_trace!(
				"tools::git_branches: branch_type={} contains={:?} not_contains={:?}",
				branch_type,
				contains,
				not_contains
			);

			let resp = call_actor(&repo, |respond| RepoCommand::GetBranches {
				branch_type: branch_type.clone(),
				contains: contains.map(|s| s.to_string()),
				not_contains: not_contains.map(|s| s.to_string()),
				respond,
			})?;
			match resp {
				RepoResponse::Branches(branches) => {
					log_trace!("tools::git_branches: {} branches", branches.len());
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
				_ => Err(unexpected("git_branches")),
			}
		}),
	);
}
