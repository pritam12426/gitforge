use std::collections::HashMap;

use serde_json::json;

use super::types::{JsonRpcRequest, JsonRpcResponse, is_notification};
use crate::error::GitforgeError;

type ToolHandler =
	Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, GitforgeError> + Send>;

struct Tool {
	handler: ToolHandler,
	description: String,
	input_schema: serde_json::Value,
}

pub struct Router {
	tools: HashMap<String, Tool>,
}

impl Router {
	pub fn new() -> Self {
		Router {
			tools: HashMap::new(),
		}
	}

	pub fn add_tool(
		&mut self,
		name: &str,
		description: &str,
		input_schema: serde_json::Value,
		handler: ToolHandler,
	) {
		self.tools.insert(
			name.into(),
			Tool {
				handler,
				description: description.into(),
				input_schema,
			},
		);
	}

	pub fn handle(&self, request: JsonRpcRequest) -> JsonRpcResponse {
		let is_notif = is_notification(&request);
		let id = request.id;

		let result = match request.method.as_str() {
			"initialize" => self.handle_initialize(),
			"ping" => Ok(json!({})),
			"tools/list" => self.handle_tools_list(),
			"tools/call" => self.handle_tools_call(request.params),
			"notifications/initialized" | "notifications/cancelled" => {
				return JsonRpcResponse::notification();
			}
			_ => Err(format!("unknown method: {}", request.method)),
		};

		if is_notif {
			return JsonRpcResponse::notification();
		}

		match result {
			Ok(value) => JsonRpcResponse::success(id, value),
			Err(msg) => JsonRpcResponse::error(id, -32601, msg),
		}
	}

	fn handle_initialize(&self) -> Result<serde_json::Value, String> {
		let tool_list: Vec<serde_json::Value> = self
			.tools
			.iter()
			.map(|(name, tool)| {
				json!({
					"name": name,
					"description": tool.description,
					"inputSchema": tool.input_schema,
				})
			})
			.collect();

		Ok(json!({
			"protocolVersion": "2025-11-25",
			"capabilities": {
				"tools": {
					"listChanged": false
				}
			},
			"serverInfo": {
				"name": "gitforge",
				"version": env!("CARGO_PKG_VERSION"),
			},
			"tools": tool_list,
		}))
	}

	fn handle_tools_list(&self) -> Result<serde_json::Value, String> {
		let tool_list: Vec<serde_json::Value> = self
			.tools
			.iter()
			.map(|(name, tool)| {
				json!({
					"name": name,
					"description": tool.description,
					"inputSchema": tool.input_schema,
				})
			})
			.collect();

		Ok(json!({ "tools": tool_list }))
	}

	fn handle_tools_call(
		&self,
		params: Option<serde_json::Value>,
	) -> Result<serde_json::Value, String> {
		let params = params.ok_or_else(|| "missing params".to_string())?;
		let name = params
			.get("name")
			.and_then(|v| v.as_str())
			.ok_or_else(|| "missing tool name".to_string())?;
		let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

		let tool = self
			.tools
			.get(name)
			.ok_or_else(|| format!("unknown tool: {}", name))?;
		let result = (tool.handler)(arguments).map_err(|e| e.to_string())?;

		Ok(json!({
			"content": [{
				"type": "text",
				"text": result
			}]
		}))
	}
}
