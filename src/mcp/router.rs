use std::collections::HashMap;

use serde_json::json;

use super::types::{is_notification, JsonRpcRequest, JsonRpcResponse};
use crate::error::GitforgeError;
use crate::git::{RepoCommand, RepoHandle, RepoResponse};

type ToolHandler =
	Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, GitforgeError> + Send>;

struct Tool {
	handler: ToolHandler,
	description: String,
	input_schema: serde_json::Value,
}

#[derive(Clone)]
pub struct Resource {
	pub uri: String,
	pub name: String,
	pub description: String,
	pub mime_type: String,
}

pub struct Router {
	tools: HashMap<String, Tool>,
	resources: Vec<Resource>,
	repo: RepoHandle,
}

impl Router {
	pub fn new(repo: RepoHandle) -> Self {
		let mut router = Router {
			tools: HashMap::new(),
			resources: Vec::new(),
			repo,
		};
		router.register_builtin_resources();
		router
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

	fn register_builtin_resources(&mut self) {
		self.resources.push(Resource {
			uri: "git://HEAD".into(),
			name: "HEAD commit".into(),
			description: "Current HEAD commit details (hash, author, message)".into(),
			mime_type: "text/plain".into(),
		});
		self.resources.push(Resource {
			uri: "git://status".into(),
			name: "Working tree status".into(),
			description: "Current working tree status".into(),
			mime_type: "text/plain".into(),
		});
	}

	pub fn handle(&self, request: JsonRpcRequest) -> JsonRpcResponse {
		let is_notif = is_notification(&request);
		let id = request.id;

		let result = match request.method.as_str() {
			"initialize" => self.handle_initialize(),
			"ping" => Ok(json!({})),
			"tools/list" => self.handle_tools_list(),
			"tools/call" => self.handle_tools_call(request.params),
			"resources/list" => self.handle_resources_list(),
			"resources/read" => self.handle_resources_read(request.params),
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

		let resource_list: Vec<serde_json::Value> = self
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

		Ok(json!({
			"protocolVersion": "2025-11-25",
			"capabilities": {
				"tools": {
					"listChanged": false
				},
				"resources": {}
			},
			"serverInfo": {
				"name": "gitforge",
				"version": env!("CARGO_PKG_VERSION"),
			},
			"tools": tool_list,
			"resources": resource_list,
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

	fn handle_resources_list(&self) -> Result<serde_json::Value, String> {
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

	fn handle_resources_read(
		&self,
		params: Option<serde_json::Value>,
	) -> Result<serde_json::Value, String> {
		let params = params.ok_or_else(|| "missing params".to_string())?;
		let uri = params
			.get("uri")
			.and_then(|v| v.as_str())
			.ok_or_else(|| "missing uri".to_string())?;

		let resource = self
			.resources
			.iter()
			.find(|r| r.uri == uri)
			.ok_or_else(|| format!("unknown resource: {}", uri))?;

		let text = self.fetch_resource_content(uri)?;

		Ok(json!({
			"contents": [{
				"uri": resource.uri,
				"mimeType": resource.mime_type,
				"text": text,
			}]
		}))
	}

	fn fetch_resource_content(&self, uri: &str) -> Result<String, String> {
		match uri {
			"git://HEAD" => self.fetch_head_info(),
			"git://status" => self.fetch_status(),
			_ => Err(format!("unknown resource: {}", uri)),
		}
	}

	fn fetch_head_info(&self) -> Result<String, String> {
		let (tx, rx) = std::sync::mpsc::channel();
		self.repo
			.send(RepoCommand::ShowCommit {
				revision: "HEAD".into(),
				respond: tx,
			})
			.map_err(|e| e.to_string())?;
		let resp = rx.recv().map_err(|_| "channel closed".to_string())?;
		match resp.map_err(|e| e.to_string())? {
			RepoResponse::ShowCommit(info) => {
				let hash = info["hash"].as_str().unwrap_or("");
				let author = info["author"].as_str().unwrap_or("");
				let email = info["email"].as_str().unwrap_or("");
				let time = info["time"].as_i64().unwrap_or(0);
				let message = info["message"].as_str().unwrap_or("");

			#[cfg(feature = "show_time_stamp")]
			let datetime = {
				let naive = std::time::UNIX_EPOCH
					+ std::time::Duration::from_secs(time as u64);
				let datetime: chrono::DateTime<chrono::Local> = naive.into();
				format!("{}", datetime.format("%a %b %e %H:%M:%S %Y"))
			};
			#[cfg(not(feature = "show_time_stamp"))]
			let datetime = time.to_string();

			let mut out = String::new();
			out.push_str(&format!("commit {}\n", hash));
			out.push_str(&format!("Author: {} <{}>\n", author, email));
			out.push_str(&format!("Date:   {}\n", datetime));
			out.push('\n');
			out.push_str(message);
			out.push('\n');
			Ok(out)
			}
			_ => Err("unexpected response".into()),
		}
	}

	fn fetch_status(&self) -> Result<String, String> {
		let (tx, rx) = std::sync::mpsc::channel();
		self.repo
			.send(RepoCommand::GetStatus { respond: tx })
			.map_err(|e| e.to_string())?;
		let resp = rx.recv().map_err(|_| "channel closed".to_string())?;
		match resp.map_err(|e| e.to_string())? {
			RepoResponse::Status(entries) => {
				if entries.is_empty() {
					Ok("nothing to commit, working tree clean".into())
				} else {
					let lines: Vec<String> = entries
						.iter()
						.map(|(path, status)| format!("{}  {}", status, path))
						.collect();
					Ok(lines.join("\n"))
				}
			}
			_ => Err("unexpected response".into()),
		}
	}
}
