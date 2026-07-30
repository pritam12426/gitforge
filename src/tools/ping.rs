use serde_json::json;

use crate::log_trace;
use crate::mcp::{Router, ToolAnnotations};

pub fn register(router: &mut Router) {
	log_trace!("tools: registering ping");
	router.add_tool(
		"ping",
		"Check if the server is alive",
		json!({ "type": "object", "properties": {} }),
		ToolAnnotations::read_only(),
		Box::new(|_| {
			log_trace!("tools::ping: handling request");
			Ok(json!("pong"))
		}),
	);
}
