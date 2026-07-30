use std::collections::HashMap;

use serde_json::json;

use super::resources::{self, Resource};
use super::types::{is_notification, JsonRpcRequest, JsonRpcResponse};
use crate::error::GitforgeError;
use crate::git::RepoHandle;
use crate::logging::truncate_for_log;
use crate::{log_debug, log_error, log_info, log_warn};

pub type ToolHandler =
	Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, GitforgeError> + Send + Sync>;

struct Tool {
	handler: ToolHandler,
	description: String,
	input_schema: serde_json::Value,
}

pub struct Router {
	tools: HashMap<String, Tool>,
	resources: Vec<Resource>,
	repo: RepoHandle,
}

impl Router {
	pub fn new(repo: RepoHandle) -> Self {
		Router { tools: HashMap::new(), resources: resources::builtin_resources(), repo }
	}

	pub fn add_tool(
		&mut self,
		name: &str,
		description: &str,
		input_schema: serde_json::Value,
		handler: ToolHandler,
	) {
		log_debug!("router: registered tool '{}'", name);
		self.tools.insert(
			name.into(),
			Tool { handler, description: description.into(), input_schema },
		);
	}

	/// Handles one JSON-RPC request. `req_id` is a correlation id
	/// allocated by the transport (stdio line), used only
	/// for logging so a single request's log lines can be grepped
	/// together across transport -> router -> actor.
	pub fn handle(&self, request: JsonRpcRequest, req_id: u64) -> JsonRpcResponse {
		let is_notif = is_notification(&request);
		let id = request.id.clone();

		log_info!("router[req={}]: dispatching method '{}'", req_id, request.method);

		let result = match request.method.as_str() {
			"initialize" => self.handle_initialize(),
			"ping" => Ok(json!({})),
			"tools/list" => self.handle_tools_list(),
			"tools/call" => self.handle_tools_call(request.params, req_id),
			"resources/list" => self.handle_resources_list(),
			"resources/read" => self.handle_resources_read(request.params, req_id),
			"notifications/initialized" | "notifications/cancelled" => {
				log_debug!("router[req={}]: notification '{}' — no response", req_id, request.method);
				return JsonRpcResponse::notification();
			}
			other => {
				log_warn!("router[req={}]: unknown method '{}'", req_id, other);
				Err(GitforgeError::NotFound(format!("unknown method: {}", other)))
			}
		};

		if is_notif {
			return JsonRpcResponse::notification();
		}

		match result {
			Ok(value) => {
				log_info!("router[req={}]: '{}' succeeded", req_id, request.method);
				JsonRpcResponse::success(id, value)
			}
			Err(e) => {
				log_error!("router[req={}]: '{}' failed: {}", req_id, request.method, e);
				JsonRpcResponse::error(id, e.rpc_code(), e.to_string())
			}
		}
	}

	fn handle_initialize(&self) -> Result<serde_json::Value, GitforgeError> {
		Ok(json!({
			"protocolVersion": "2025-11-25",
			"capabilities": {
				"tools": { "listChanged": false },
				"resources": {}
			},
			"serverInfo": {
				"name": "gitforge",
				"version": env!("CARGO_PKG_VERSION"),
			},
			"tools": self.tool_list_json(),
			"resources": self.resource_list_json(),
		}))
	}

	fn handle_tools_list(&self) -> Result<serde_json::Value, GitforgeError> {
		Ok(json!({ "tools": self.tool_list_json() }))
	}

	fn tool_list_json(&self) -> Vec<serde_json::Value> {
		self.tools
			.iter()
			.map(|(name, tool)| {
				json!({
					"name": name,
					"description": tool.description,
					"inputSchema": tool.input_schema,
				})
			})
			.collect()
	}

	fn handle_tools_call(
		&self,
		params: Option<serde_json::Value>,
		req_id: u64,
	) -> Result<serde_json::Value, GitforgeError> {
		let params = params.ok_or_else(|| GitforgeError::InvalidRequest("missing params".into()))?;
		let name = params
			.get("name")
			.and_then(|v| v.as_str())
			.ok_or_else(|| GitforgeError::InvalidRequest("missing tool name".into()))?;
		let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

		log_info!(
			"tool[req={}]: calling '{}' args={}",
			req_id,
			name,
			truncate_for_log(&arguments.to_string(), 300)
		);

		let tool = self
			.tools
			.get(name)
			.ok_or_else(|| GitforgeError::NotFound(format!("unknown tool: {}", name)))?;

		let result = (tool.handler)(arguments);

		match &result {
			Ok(value) => log_info!(
				"tool[req={}]: '{}' returned: {}",
				req_id,
				name,
				truncate_for_log(&value.to_string(), 300)
			),
			Err(e) => log_error!("tool[req={}]: '{}' errored: {}", req_id, name, e),
		}

		let value = result?;
		Ok(json!({ "content": [{ "type": "text", "text": value }] }))
	}

	fn handle_resources_list(&self) -> Result<serde_json::Value, GitforgeError> {
		let list: Vec<serde_json::Value> = self
			.resources
			.iter()
			.map(|r| {
				json!({
					"uri": r.uri,
					"name": r.name,
					"description": r.description,
					"mimeType": r.mime_type,
				})
			})
			.collect();
		Ok(json!({ "resources": list }))
	}

	fn resource_list_json(&self) -> Vec<serde_json::Value> {
		self.resources
			.iter()
			.map(|r| {
				json!({
					"uri": r.uri,
					"name": r.name,
					"description": r.description,
					"mimeType": r.mime_type,
				})
			})
			.collect()
	}

	fn handle_resources_read(
		&self,
		params: Option<serde_json::Value>,
		req_id: u64,
	) -> Result<serde_json::Value, GitforgeError> {
		let params = params.ok_or_else(|| GitforgeError::InvalidRequest("missing params".into()))?;
		let uri = params
			.get("uri")
			.and_then(|v| v.as_str())
			.ok_or_else(|| GitforgeError::InvalidRequest("missing uri".into()))?;

		log_info!("resource[req={}]: reading '{}'", req_id, uri);

		let resource = self
			.resources
			.iter()
			.find(|r| r.uri == uri)
			.ok_or_else(|| GitforgeError::NotFound(format!("unknown resource: {}", uri)))?;

		let text = resources::fetch_content(&self.repo, uri)?;

		Ok(json!({
			"contents": [{
				"uri": resource.uri,
				"mimeType": resource.mime_type,
				"text": text,
			}]
		}))
	}
}
