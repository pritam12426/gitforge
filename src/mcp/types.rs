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

	pub fn notification() -> Self {
		JsonRpcResponse {
			jsonrpc: "2.0".into(),
			id: None,
			result: None,
			error: None,
		}
	}
}

pub fn is_notification(request: &JsonRpcRequest) -> bool {
	request.method.starts_with("notifications/") || request.id.is_none()
}
