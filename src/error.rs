//! Error types for gitforge.
//!
//! One top-level enum (`GitforgeError`) is still what flows through the
//! actor channel and tool handlers — that hasn't changed from the original
//! design, since a single error type is what makes the `?` operator work
//! smoothly across git2/io/serde boundaries. What changed is that the
//! catch-all `Internal(String)` variant has been split into more specific
//! variants so callers (the router, the transports) can decide *how* to
//! surface an error — e.g. "unknown tool" should become a JSON-RPC
//! "method not found" style error, while "merge conflict" is a normal
//! tool-execution failure — without string-matching error messages.

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

	/// HTTP transport setup/runtime error (bind failure, etc).
	#[error("http transport error: {0}")]
	Http(String),

	#[error("internal error: {0}")]
	Internal(String),
}

impl GitforgeError {
	/// Maps this error onto a JSON-RPC 2.0 error code. Used by both the
	/// stdio and HTTP transports so the two stay in sync.
	pub fn rpc_code(&self) -> i32 {
		match self {
			GitforgeError::NotFound(_) => -32601, // Method not found
			GitforgeError::InvalidRequest(_) => -32602, // Invalid params
			GitforgeError::Serde(_) => -32700,    // Parse error
			_ => -32000,                          // Server error (generic)
		}
	}
}
