use std::io::{BufRead, Write};

use serde_json::json;

use crate::error::GitforgeError;
use crate::logging::{next_request_id, truncate_for_log};
use crate::mcp::Router;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, is_notification};
use crate::{log_debug, log_error, log_info, log_trace, log_warn};

/// Reads JSON-RPC 2.0 requests from stdin, dispatches them through the
/// router, and writes responses to stdout.
///
/// Uses a manual `read_line` loop (rather than `stdin.lock().lines()`)
/// so that the stdin lock can be shared with [`send_roots_request`],
/// which needs to read a second response mid-stream after `initialize`.
pub fn run(router: &Router) -> Result<(), GitforgeError> {
	log_info!("transport(stdio): starting read loop");
	let stdin = std::io::stdin();
	let stdout = std::io::stdout();
	let mut reader = stdin.lock();
	let mut out = stdout.lock();

	let mut line = String::new();
	loop {
		line.clear();
		if reader.read_line(&mut line)? == 0 {
			break;
		}
		let trimmed = line.trim();
		if trimmed.is_empty() {
			log_trace!("transport(stdio): skipping empty line");
			continue;
		}

		let req_id = next_request_id();
		log_debug!(
			"transport(stdio)[req={}]: <- {}",
			req_id,
			truncate_for_log(trimmed, 300)
		);

		let request: JsonRpcRequest = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
			Ok(r) => {
				log_trace!(
					"transport(stdio)[req={}]: parsed method='{}'",
					req_id,
					r.method
				);
				r
			}
			Err(e) => {
				log_warn!("transport(stdio)[req={}]: parse error: {}", req_id, e);
				let resp = JsonRpcResponse::error(None, -32700, format!("parse error: {}", e));
				write_response(&mut out, &resp, req_id)?;
				continue;
			}
		};

		let is_initialize = request.method == "initialize";
		let is_notif = is_notification(&request);

		let response = router.handle(request, req_id);

		if response.is_notification_sentinel() {
			log_debug!(
				"transport(stdio)[req={}]: notification, no reply sent",
				req_id
			);
			continue;
		}

		write_response(&mut out, &response, req_id)?;

		// MCP Roots discovery: after the initialize handshake, if the
		// client declared `roots` capability, the server may send a
		// `roots/list` request to discover repositories the client
		// manages.  We do this synchronously right after the initialize
		// response, blocking the main loop until the reply arrives.
		// This is simpler (and common in practice) than a fully
		// asynchronous request/response multiplexer, and it guarantees
		// roots are available before any tool call is dispatched.
		if is_initialize && !is_notif && router.client_supports_roots() {
			send_roots_request(&mut reader, &mut out, router, req_id)?;
		}
	}

	log_info!("transport(stdio): stdin closed, shutting down");
	Ok(())
}

fn send_roots_request(
	reader: &mut std::io::StdinLock<'_>,
	out: &mut std::io::StdoutLock<'_>,
	router: &Router,
	req_id: u64,
) -> Result<(), GitforgeError> {
	let roots_req = json!({
		"jsonrpc": "2.0",
		"id": 0,
		"method": "roots/list"
	});
	let json = serde_json::to_string(&roots_req)?;
	log_debug!(
		"transport(stdio)[req={}]: -> (roots/list) {}",
		req_id,
		truncate_for_log(&json, 300)
	);
	writeln!(out, "{}", json)?;
	out.flush()?;

	let mut line = String::new();
	if reader.read_line(&mut line)? == 0 {
		return Err(GitforgeError::Io(std::io::Error::new(
			std::io::ErrorKind::UnexpectedEof,
			"expected roots/list response",
		)));
	}
	let trimmed = line.trim();
	log_debug!(
		"transport(stdio)[req={}]: <- (roots/list response) {}",
		req_id,
		truncate_for_log(trimmed, 300)
	);

	let resp: serde_json::Value = serde_json::from_str(trimmed)?;
	if let Some(result) = resp.get("result") {
		if let Some(roots) = result.get("roots").and_then(|r| r.as_array()) {
			let paths: Vec<String> = roots
				.iter()
				.filter_map(|r| {
					let uri = r.get("uri").and_then(|u| u.as_str())?;
					// Convert file:// URI to local path, or use as-is
					if let Some(path) = uri.strip_prefix("file://") {
						Some(url_to_path(path))
					} else if uri.starts_with('/') {
						Some(uri.to_string())
					} else {
						None
					}
				})
				.collect();
			router.set_client_roots(paths);
			log_info!(
				"transport(stdio)[req={}]: roots discovery returned {} repo(s)",
				req_id,
				router.client_roots().map_or(0, |r| r.len())
			);
		}
	}

	Ok(())
}

/// Simple file:// URI path decoder (handles percent-encoded chars).
fn url_to_path(uri: &str) -> String {
	// Percent-decode common encodings
	let mut decoded = String::with_capacity(uri.len());
	let mut chars = uri.chars();
	while let Some(c) = chars.next() {
		if c == '%' {
			let hex: String = chars.by_ref().take(2).collect();
			if let Ok(byte) = u8::from_str_radix(&hex, 16) {
				decoded.push(byte as char);
			} else {
				decoded.push('%');
				decoded.push_str(&hex);
			}
		} else {
			decoded.push(c);
		}
	}
	decoded
}

fn write_response(
	out: &mut std::io::StdoutLock<'_>,
	response: &JsonRpcResponse,
	req_id: u64,
) -> Result<(), GitforgeError> {
	let json = serde_json::to_string(response)?;
	log_debug!(
		"transport(stdio)[req={}]: -> {}",
		req_id,
		truncate_for_log(&json, 300)
	);

	if let Err(e) = writeln!(out, "{}", json) {
		log_error!("transport(stdio)[req={}]: write failed: {}", req_id, e);
		return Err(e.into());
	}
	out.flush()?;
	Ok(())
}
