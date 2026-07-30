use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct JsonRpcRequest {
	#[serde(default = "default_jsonrpc")]
	pub jsonrpc: String,
	pub id: Option<serde_json::Value>,
	pub method: String,
	#[serde(default)]
	pub params: Option<serde_json::Value>,
}

fn default_jsonrpc() -> String {
	"2.0".into()
}

#[derive(Serialize)]
pub struct JsonRpcResponse {
	pub jsonrpc: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<serde_json::Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<serde_json::Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<JsonRpcError>,
}

#[derive(Serialize)]
pub struct JsonRpcError {
	pub code: i32,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
	pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
		JsonRpcResponse {
			jsonrpc: "2.0".into(),
			id,
			result: Some(result),
			error: None,
		}
	}

	pub fn error(id: Option<serde_json::Value>, code: i32, message: impl Into<String>) -> Self {
		JsonRpcResponse {
			jsonrpc: "2.0".into(),
			id,
			result: None,
			error: Some(JsonRpcError {
				code,
				message: message.into(),
				data: None,
			}),
		}
	}

	/// A JSON-RPC notification never gets a reply. Transports use this
	/// sentinel to recognise "produce no output" without threading an
	/// `Option<JsonRpcResponse>` through every call site.
	pub fn notification() -> Self {
		JsonRpcResponse {
			jsonrpc: "2.0".into(),
			id: None,
			result: None,
			error: None,
		}
	}

	pub fn is_notification_sentinel(&self) -> bool {
		self.id.is_none() && self.result.is_none() && self.error.is_none()
	}
}

/// Returns `true` if the request is a JSON-RPC notification (no response
/// expected).  An `id` of `None` marks a notification per spec, but we
/// also treat `notifications/*` methods as notifications regardless of
/// their id — the MCP spec says these standard notifications should not
/// receive a reply.
pub fn is_notification(request: &JsonRpcRequest) -> bool {
	request.method.starts_with("notifications/") || request.id.is_none()
}
