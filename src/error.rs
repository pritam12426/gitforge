use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitforgeError {
	#[error("Git error: {0}")]
	Git(#[from] git2::Error),

	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error("Serialization error: {0}")]
	Serde(#[from] serde_json::Error),

	#[error("MCP error: {0}")]
	Mcp(String),

	#[error("Internal error: {0}")]
	Internal(String),

	#[error("Channel closed")]
	ChannelClosed,
}
