use serde_json::json;

use crate::mcp::Router;

pub fn register(router: &mut Router) {
	router.add_tool(
		"ping",
		"Check if the server is alive",
		json!({ "type": "object", "properties": {} }),
		Box::new(|_| Ok(json!("pong"))),
	);
}
