//! Error types for gitforge.
//!
//! One top-level enum (`GitforgeError`) flows through the actor channel
//! and tool handlers. The specific variants let callers (the router, the
//! transports) surface errors with appropriate JSON-RPC codes without
//! string-matching error messages.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitforgeError {
	#[error("git error: {0}")]
	Git(#[from] git2::Error),

	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error("serialization error: {0}")]
	Serde(#[from] serde_json::Error),

	/// A JSON-RPC request was missing required fields or had the wrong
	/// shape (missing params, wrong param type, missing tool name, ...).
	#[error("invalid request: {0}")]
	InvalidRequest(String),

	/// A named tool, resource, or method does not exist.
	#[error("not found: {0}")]
	NotFound(String),

	/// The requested git operation is well-formed but cannot complete
	/// for domain reasons (merge conflict, duplicate branch name, ...).
	#[error("operation failed: {0}")]
	OperationFailed(String),

	/// The repo actor thread is gone (channel disconnected) or a
	/// request to it timed out.
	#[error("actor unavailable: {0}")]
	Actor(String),

	#[error("internal error: {0}")]
	Internal(String),
}

impl GitforgeError {
	/// Maps this error onto a JSON-RPC 2.0 error code.
	pub fn rpc_code(&self) -> i32 {
		match self {
			GitforgeError::NotFound(_) => -32601,     // Method not found
			GitforgeError::InvalidRequest(_) => -32602, // Invalid params
			GitforgeError::Serde(_) => -32700,        // Parse error
			_ => -32000,                              // Server error (generic)
		}
	}
}
