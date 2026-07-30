use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use serde::Serialize;
use serde_json::json;

use super::resources::{self, Resource};
use super::types::{is_notification, JsonRpcRequest, JsonRpcResponse};
use crate::error::GitforgeError;
use crate::git::RepoHandle;
use crate::logging::truncate_for_log;
use crate::{log_debug, log_error, log_info, log_trace, log_warn};

pub type ToolHandler =
	Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, GitforgeError> + Send + Sync>;

#[derive(Clone, Serialize)]
pub struct ToolAnnotations {
	pub read_only_hint: bool,
	pub destructive_hint: bool,
	pub idempotent_hint: bool,
	pub open_world_hint: bool,
}

impl ToolAnnotations {
	pub const fn read_only() -> Self {
		ToolAnnotations {
			read_only_hint: true,
			destructive_hint: false,
			idempotent_hint: true,
			open_world_hint: false,
		}
	}

	pub const fn destructive() -> Self {
		ToolAnnotations {
			read_only_hint: false,
			destructive_hint: true,
			idempotent_hint: true,
			open_world_hint: false,
		}
	}

	pub const fn mutable() -> Self {
		ToolAnnotations {
			read_only_hint: false,
			destructive_hint: false,
			idempotent_hint: false,
			open_world_hint: false,
		}
	}
}

struct Tool {
	handler: ToolHandler,
	description: String,
	input_schema: serde_json::Value,
	annotations: ToolAnnotations,
}

pub struct Router {
	tools: HashMap<String, Tool>,
	resources: Vec<Resource>,
	repo: RepoHandle,
	allowed_repo: Option<std::path::PathBuf>,
	client_roots: RefCell<Option<Vec<String>>>,
	client_supports_roots: Cell<bool>,
}

impl Router {
	pub fn new(repo: RepoHandle, allowed_repo: Option<std::path::PathBuf>) -> Self {
		Router {
			tools: HashMap::new(),
			resources: resources::builtin_resources(),
			repo,
			allowed_repo,
			client_roots: RefCell::new(None),
			client_supports_roots: Cell::new(false),
		}
	}

	pub fn add_tool(
		&mut self,
		name: &str,
		description: &str,
		input_schema: serde_json::Value,
		annotations: ToolAnnotations,
		handler: ToolHandler,
	) {
		log_debug!("router: registered tool '{}'", name);
		self.tools.insert(
			name.into(),
			Tool { handler, description: description.into(), input_schema, annotations },
		);
	}

	pub fn allowed_repo(&self) -> Option<&std::path::Path> {
		self.allowed_repo.as_deref()
	}

	pub fn client_supports_roots(&self) -> bool {
		self.client_supports_roots.get()
	}

	pub fn set_client_roots(&self, roots: Vec<String>) {
		self.client_roots.replace(Some(roots));
	}

	pub fn client_roots(&self) -> Option<Vec<String>> {
		self.client_roots.borrow().clone()
	}

	/// Handles one JSON-RPC request. `req_id` is a correlation id
	/// allocated by the transport (stdio line), used only
	/// for logging so a single request's log lines can be grepped
	/// together across transport -> router -> actor.
	pub fn handle(&self, request: JsonRpcRequest, req_id: u64) -> JsonRpcResponse {
		let is_notif = is_notification(&request);
		let id = request.id.clone();

		log_info!("router[req={}]: dispatching method '{}'", req_id, request.method);
		log_trace!("router[req={}]: params={}", req_id, truncate_for_log(&format!("{:?}", request.params), 200));

		let result = match request.method.as_str() {
			"initialize" => self.handle_initialize(&request),
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

	fn handle_initialize(&self, request: &JsonRpcRequest) -> Result<serde_json::Value, GitforgeError> {
		// Extract client capabilities to check for roots support
		if let Some(params) = &request.params {
			if let Some(caps) = params.get("capabilities") {
				if caps.get("roots").and_then(|r| r.as_object()).is_some() {
					self.client_supports_roots.set(true);
					log_info!("router: client supports roots capability");
				}
			}
		}

		log_trace!("router: building initialize response with {} tools", self.tools.len());
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
		log_trace!("router: tools/list — {} tools", self.tools.len());
		Ok(json!({ "tools": self.tool_list_json() }))
	}

	fn tool_list_json(&self) -> Vec<serde_json::Value> {
		let tools: Vec<serde_json::Value> = self
			.tools
			.iter()
			.map(|(name, tool)| {
				json!({
					"name": name,
					"description": tool.description,
					"inputSchema": tool.input_schema,
					"annotations": tool.annotations,
				})
			})
			.collect();
		log_trace!("router: tool_list_json — serialized {} tools", tools.len());
		tools
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
		log_trace!("router: resources/list — {} resources", self.resources.len());
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

		log_trace!("router[req={}]: looking up resource '{}'", req_id, uri);
		let resource = self
			.resources
			.iter()
			.find(|r| r.uri == uri)
			.ok_or_else(|| GitforgeError::NotFound(format!("unknown resource: {}", uri)))?;

		log_trace!("router[req={}]: fetching content for '{}'", req_id, uri);
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
