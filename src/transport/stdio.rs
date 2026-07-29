use std::io::{BufRead, Write};

use crate::error::GitforgeError;
use crate::logging::{next_request_id, truncate_for_log};
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::Router;
use crate::{log_debug, log_error, log_info, log_warn};

pub fn run(router: &Router) -> Result<(), GitforgeError> {
	log_info!("transport(stdio): starting read loop");
	let stdin = std::io::stdin();
	let stdout = std::io::stdout();

	for line in stdin.lock().lines() {
		let line = line?;
		if line.trim().is_empty() {
			continue;
		}

		let req_id = next_request_id();
		log_debug!(
			"transport(stdio)[req={}]: <- {}",
			req_id,
			truncate_for_log(&line, 300)
		);

		let request: JsonRpcRequest = match serde_json::from_str(&line) {
			Ok(r) => r,
			Err(e) => {
				log_warn!("transport(stdio)[req={}]: parse error: {}", req_id, e);
				let resp = JsonRpcResponse::error(None, -32700, format!("parse error: {}", e));
				write_response(&stdout, &resp, req_id)?;
				continue;
			}
		};

		let response = router.handle(request, req_id);

		if response.is_notification_sentinel() {
			log_debug!("transport(stdio)[req={}]: notification, no reply sent", req_id);
			continue;
		}

		write_response(&stdout, &response, req_id)?;
	}

	log_info!("transport(stdio): stdin closed, shutting down");
	Ok(())
}

fn write_response(
	stdout: &std::io::Stdout,
	response: &JsonRpcResponse,
	req_id: u64,
) -> Result<(), GitforgeError> {
	let json = serde_json::to_string(response)?;
	log_debug!("transport(stdio)[req={}]: -> {}", req_id, truncate_for_log(&json, 300));

	let mut out = stdout.lock();
	if let Err(e) = writeln!(out, "{}", json) {
		log_error!("transport(stdio)[req={}]: write failed: {}", req_id, e);
		return Err(e.into());
	}
	out.flush()?;
	Ok(())
}
