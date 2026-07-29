use std::io::{BufRead, Write};

use super::router::Router;
use super::types::{JsonRpcRequest, JsonRpcResponse};
use crate::error::GitforgeError;

pub fn run_stdio_loop(router: &Router) -> Result<(), GitforgeError> {
	let stdin = std::io::stdin();
	let stdout = std::io::stdout();

	for line in stdin.lock().lines() {
		let line = line?;
		if line.trim().is_empty() {
			continue;
		}

		let request: JsonRpcRequest = match serde_json::from_str(&line) {
			Ok(r) => r,
			Err(e) => {
				let resp = JsonRpcResponse::error(None, -32700, format!("parse error: {}", e));
				let json = serde_json::to_string(&resp)?;
				let mut out = stdout.lock();
				writeln!(out, "{}", json)?;
				out.flush()?;
				continue;
			}
		};

		let response = router.handle(request);

		if response.id.is_none() && response.result.is_none() && response.error.is_none() {
			continue;
		}

		let json = serde_json::to_string(&response)?;
		let mut out = stdout.lock();
		writeln!(out, "{}", json)?;
		out.flush()?;
	}

	Ok(())
}
