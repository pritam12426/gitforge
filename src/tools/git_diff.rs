/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

use serde_json::json;

use crate::git::{RepoCommand, RepoHandle, RepoResponse};
use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

use super::{call_actor, reject_flag, required_str, unexpected};

pub fn register(router: &mut Router, repo: RepoHandle) {
	log_trace!("tools: registering git_diff");
	router.add_tool(
		"git_diff",
		"Show differences between HEAD and a target branch or commit",
		json!({
			"type": "object",
			"properties": {
				"target": {
					"type": "string",
					"description": "Target branch, commit SHA, or ref to compare HEAD against"
				}
			},
			"required": ["target"]
		}),
		ToolAnnotations::read_only(),
		Box::new(move |args| {
			let target = required_str(&args, "target")?;
			reject_flag(&target, "target")?;
			log_trace!("tools::git_diff: target='{}'", target);

			let resp = call_actor(&repo, |respond| RepoCommand::GetDiffTarget {
				target,
				respond,
			})?;
			match resp {
				RepoResponse::Diff(text) => {
					log_trace!("tools::git_diff: {} bytes diff output", text.len());
					if text.is_empty() {
						Ok(json!("no differences"))
					} else {
						Ok(json!(text))
					}
				}
				_ => Err(unexpected("git_diff")),
			}
		}),
	);
}
