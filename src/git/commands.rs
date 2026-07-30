use std::sync::mpsc;

use crate::error::GitforgeError;

pub type Respond = mpsc::Sender<Result<RepoResponse, GitforgeError>>;

#[derive(Debug)]
pub enum RepoResponse {
	Health,
	Status(Vec<(String, String)>),
	Log(Vec<(String, String, String)>),
	Branches(Vec<(String, bool)>),
	Diff(String),
	ShowCommit(serde_json::Value),
	CommitCreated(String),
	Staged,
	BranchCreated(String),
	CheckoutOk,
	MergeOk(String),
}

pub enum RepoCommand {
	CheckHealth {
		respond: Respond,
	},
	GetStatus {
		respond: Respond,
	},
	GetLog {
		offset: usize,
		max_count: usize,
		respond: Respond,
	},
	GetBranches {
		branch_type: String,
		contains: Option<String>,
		not_contains: Option<String>,
		respond: Respond,
	},
	GetDiffUnstaged {
		respond: Respond,
	},
	GetDiffStaged {
		respond: Respond,
	},
	GetDiffTarget {
		target: String,
		respond: Respond,
	},
	ShowCommit {
		revision: String,
		respond: Respond,
	},
	CreateCommit {
		message: String,
		author_name: String,
		author_email: String,
		respond: Respond,
	},
	StageFiles {
		paths: Vec<String>,
		respond: Respond,
	},
	CreateBranch {
		name: String,
		revision: String,
		respond: Respond,
	},
	Checkout {
		branch: String,
		respond: Respond,
	},
	Merge {
		branch: String,
		respond: Respond,
	},
}

impl RepoCommand {
	/// Short, stable name used in lifecycle log lines — kept separate
	/// from any `Debug` derive so log output doesn't change shape if a
	/// variant's payload changes.
	pub fn name(&self) -> &'static str {
		match self {
			RepoCommand::CheckHealth { .. } => "check_health",
			RepoCommand::GetStatus { .. } => "get_status",
			RepoCommand::GetLog { .. } => "get_log",
			RepoCommand::GetBranches { .. } => "get_branches",
			RepoCommand::GetDiffUnstaged { .. } => "get_diff_unstaged",
			RepoCommand::GetDiffStaged { .. } => "get_diff_staged",
			RepoCommand::GetDiffTarget { .. } => "get_diff_target",
			RepoCommand::ShowCommit { .. } => "show_commit",
			RepoCommand::CreateCommit { .. } => "create_commit",
			RepoCommand::StageFiles { .. } => "stage_files",
			RepoCommand::CreateBranch { .. } => "create_branch",
			RepoCommand::Checkout { .. } => "checkout",
			RepoCommand::Merge { .. } => "merge",
		}
	}
}
