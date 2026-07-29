//! HTTP transport for `gitforge server`. Serves the *same* JSON-RPC
//! protocol as stdio, just over `POST /rpc` instead of stdin/stdout lines
//! — a client sends one JSON-RPC request as the body and gets one
//! JSON-RPC response back, always with HTTP 200 (JSON-RPC's own `error`
//! field carries protocol-level failures, matching how the stdio
//! transport never uses anything but a single output channel either).
//!
//! `axum` was chosen over `warp`/`actix-web` because it needs the
//! smallest footprint here (one POST route, one GET health-check route,
//! no middleware ecosystem beyond a request-logging layer) and builds on
//! `tower`, which lets `tests/integration.rs` drive it in-process via
//! `ServiceExt::oneshot` without binding a real socket.
//!
//! `Router::handle` is synchronous and can block for up to 30s waiting
//! on the repo actor (see `git::handle::recv_response`); each request is
//! therefore run inside `spawn_blocking` so a slow git operation never
//! stalls the async runtime's worker threads.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};

use crate::error::GitforgeError;
use crate::logging::{next_request_id, truncate_for_log};
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::Router as McpRouter;
use crate::{log_debug, log_error, log_info, log_warn};

pub fn build_app(router: Arc<McpRouter>) -> axum::Router {
	axum::Router::new()
		.route("/health", get(health))
		.route("/rpc", post(handle_rpc))
		.with_state(router)
}

pub async fn run(router: Arc<McpRouter>, host: &str, port: u16) -> Result<(), GitforgeError> {
	let app = build_app(router);
	let addr = format!("{host}:{port}");

	log_info!("transport(http): binding {}", addr);
	let listener = tokio::net::TcpListener::bind(&addr)
		.await
		.map_err(|e| GitforgeError::Http(format!("failed to bind {addr}: {e}")))?;

	log_info!("transport(http): listening on {}", addr);
	axum::serve(listener, app)
		.await
		.map_err(|e| GitforgeError::Http(format!("server error: {e}")))?;

	Ok(())
}

async fn health() -> &'static str {
	"ok"
}

async fn handle_rpc(State(router): State<Arc<McpRouter>>, body: String) -> impl IntoResponse {
	let req_id = next_request_id();
	log_debug!(
		"transport(http)[req={}]: <- POST /rpc {}",
		req_id,
		truncate_for_log(&body, 300)
	);

	let request: JsonRpcRequest = match serde_json::from_str(&body) {
		Ok(r) => r,
		Err(e) => {
			log_warn!("transport(http)[req={}]: parse error: {}", req_id, e);
			let resp = JsonRpcResponse::error(None, -32700, format!("parse error: {}", e));
			return respond(req_id, resp);
		}
	};

	// `Router::handle` is sync and can block on the actor channel; run it
	// on a blocking-pool thread so we never stall the async runtime.
	let response = match tokio::task::spawn_blocking(move || router.handle(request, req_id)).await
	{
		Ok(resp) => resp,
		Err(join_err) => {
			log_error!("transport(http)[req={}]: task panicked: {}", req_id, join_err);
			JsonRpcResponse::error(None, -32000, "internal error handling request")
		}
	};

	if response.is_notification_sentinel() {
		log_debug!("transport(http)[req={}]: notification — replying with empty object", req_id);
		// HTTP always needs *some* body; JSON-RPC notifications over
		// HTTP conventionally get an empty 202/200 with no content.
		return (StatusCode::OK, Json(serde_json::json!({}))).into_response();
	}

	respond(req_id, response)
}

fn respond(req_id: u64, response: JsonRpcResponse) -> axum::response::Response {
	let body = serde_json::to_string(&response).unwrap_or_else(|_| "{}".into());
	log_debug!("transport(http)[req={}]: -> {}", req_id, truncate_for_log(&body, 300));
	(StatusCode::OK, Json(response)).into_response()
}
