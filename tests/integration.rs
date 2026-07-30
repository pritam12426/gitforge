/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

//! Integration tests.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};
use tempfile::TempDir;

// ── stdio helpers ───────────────────────────────────────────────────────

/// Creates a temp repo with two commits and a configured identity, ready
/// for tools that read history (git_log, git_show, git_diff, ...).
fn setup_repo() -> TempDir {
	let dir = tempfile::tempdir().expect("tempdir");
	let mut opts = git2::RepositoryInitOptions::new();
	opts.initial_head("master");
	let repo = git2::Repository::init_opts(dir.path(), &opts).expect("git init");

	{
		let mut config = repo.config().expect("config");
		config.set_str("user.name", "Test User").unwrap();
		config.set_str("user.email", "test@example.com").unwrap();
	}

	write_and_commit(&repo, dir.path(), "README.md", "hello\n", "Initial commit");
	write_and_commit(
		&repo,
		dir.path(),
		"README.md",
		"hello\nworld\n",
		"Second commit",
	);

	dir
}

fn write_and_commit(repo: &git2::Repository, dir: &Path, file: &str, contents: &str, msg: &str) {
	std::fs::write(dir.join(file), contents).unwrap();

	let mut index = repo.index().unwrap();
	index.add_path(Path::new(file)).unwrap();
	index.write().unwrap();
	let tree_id = index.write_tree().unwrap();
	let tree = repo.find_tree(tree_id).unwrap();

	let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
	let parents: Vec<git2::Commit> = repo
		.head()
		.ok()
		.and_then(|h| h.peel_to_commit().ok())
		.into_iter()
		.collect();
	let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

	repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
		.unwrap();
}

/// Spawns the compiled binary against `dir` in stdio mode and returns
/// a session that holds the piped stdin/stdout handles.
struct TestSession {
	stdin: std::process::ChildStdin,
	reader: BufReader<std::process::ChildStdout>,
	child: Option<Child>,
}

impl TestSession {
	fn spawn(dir: &Path) -> Self {
		let mut child = Command::new(env!("CARGO_BIN_EXE_gitforge"))
			.arg("--repo")
			.arg(dir)
			.arg("--log-level")
			.arg("off")
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.expect("spawn gitforge");

		let stdin = child.stdin.take().expect("stdin");
		let stdout = child.stdout.take().expect("stdout");
		let reader = BufReader::new(stdout);

		TestSession {
			stdin,
			reader,
			child: Some(child),
		}
	}

	/// Writes each request as one line, reads back one response line per
	/// non-notification request, in order.
	fn send(&mut self, requests: &[Value]) -> Vec<Value> {
		let mut responses = Vec::new();
		for req in requests {
			let line = serde_json::to_string(req).unwrap();
			writeln!(self.stdin, "{}", line).unwrap();
			self.stdin.flush().unwrap();

			let is_notification = req.get("id").is_none()
				|| req
					.get("method")
					.and_then(|m| m.as_str())
					.map(|m| m.starts_with("notifications/"))
					.unwrap_or(false);
			if is_notification {
				continue;
			}

			let mut resp_line = String::new();
			self.reader
				.read_line(&mut resp_line)
				.expect("read response");
			responses.push(serde_json::from_str(&resp_line).expect("parse response"));
		}
		responses
	}

	fn send_one(&mut self, req: Value) -> Value {
		self.send(&[req]).into_iter().next().expect("one response")
	}
}

impl Drop for TestSession {
	fn drop(&mut self) {
		if let Some(mut child) = self.child.take() {
			let _ = child.kill();
			let _ = child.wait();
		}
	}
}

// ── stdio protocol tests ─────────────────────────────────────────────────

#[test]
fn test_ping() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "ping", "arguments": {} } }),
	);

	assert_eq!(resp["result"]["content"][0]["text"], json!("pong"));
}

#[test]
fn test_initialize_returns_tools_and_resources() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }));

	let tools = resp["result"]["tools"].as_array().expect("tools array");
	assert!(
		tools.len() >= 12,
		"expected at least 12 tools, got {}",
		tools.len()
	);
	let resources = resp["result"]["resources"]
		.as_array()
		.expect("resources array");
	assert_eq!(resources.len(), 2);
}

#[test]
fn test_unknown_method() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(json!({ "jsonrpc": "2.0", "id": 1, "method": "nonexistent" }));
	assert!(resp["error"].is_object());
	assert_eq!(resp["error"]["code"], json!(-32601));
}

#[test]
fn test_parse_error() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	// Manually send invalid JSON and read the response.
	writeln!(session.stdin, "{{not valid json").unwrap();
	session.stdin.flush().unwrap();

	let mut line = String::new();
	session.reader.read_line(&mut line).unwrap();
	let resp: Value = serde_json::from_str(&line).unwrap();
	assert_eq!(resp["error"]["code"], json!(-32700));
}

#[test]
fn test_notification_no_response() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	// A notification followed by a real request: only one response line
	// should come back, and it must be for the second request.
	let responses = session.send(&[
		json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
		json!({ "jsonrpc": "2.0", "id": 42, "method": "ping" }),
	]);
	assert_eq!(responses.len(), 1);
	assert_eq!(responses[0]["id"], json!(42));
}

#[test]
fn test_git_status_clean() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_status", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("clean"), "expected clean status, got: {text}");
}

#[test]
fn test_git_status_dirty() {
	let dir = setup_repo();
	std::fs::write(dir.path().join("untracked.txt"), "new file\n").unwrap();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_status", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(
		text.contains("untracked.txt"),
		"expected untracked file listed, got: {text}"
	);
}

#[test]
fn test_git_log_default_and_limit() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_log", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert_eq!(text.lines().count(), 2, "expected 2 commits, got: {text}");

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "git_log", "arguments": { "limit": 1 } } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert_eq!(text.lines().count(), 1);
}

#[test]
fn test_git_show_head() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_show", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("Second commit"));
}

#[test]
fn test_git_show_unknown_revision() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_show", "arguments": { "revision": "not-a-real-rev" } } }),
	);
	assert!(resp["error"].is_object());
}

#[test]
fn test_resources_list_and_read() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" }));
	assert_eq!(resp["result"]["resources"].as_array().unwrap().len(), 2);

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 2, "method": "resources/read", "params": { "uri": "git://status" } }),
	);
	assert!(
		resp["result"]["contents"][0]["text"]
			.as_str()
			.unwrap()
			.contains("clean")
	);

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/read", "params": { "uri": "git://nope" } }),
	);
	assert!(resp["error"].is_object());
}

#[test]
fn test_git_add_and_commit() {
	let dir = setup_repo();
	std::fs::write(dir.path().join("new.txt"), "content\n").unwrap();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_add", "arguments": { "files": ["new.txt"] } } }),
	);
	assert_eq!(resp["result"]["content"][0]["text"], json!("staged"));

	let resp = session.send_one(json!({
		"jsonrpc": "2.0", "id": 2, "method": "tools/call",
		"params": { "name": "git_commit", "arguments": {
			"message": "add new.txt", "author_name": "Test", "author_email": "t@example.com"
		}}
	}));
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.starts_with("Created commit"));
}

#[test]
fn test_git_branch_create_and_checkout() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_branch_create", "arguments": { "name": "feature" } } }),
	);
	assert_eq!(
		resp["result"]["content"][0]["text"],
		json!("Created branch 'feature'")
	);

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "git_checkout", "arguments": { "branch": "feature" } } }),
	);
	assert_eq!(
		resp["result"]["content"][0]["text"],
		json!("switched branch")
	);

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "git_checkout", "arguments": { "branch": "does-not-exist" } } }),
	);
	assert!(resp["error"].is_object());
}

#[test]
fn test_git_merge_fast_forward_and_already_up_to_date() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_branch_create", "arguments": { "name": "feature" } } }),
	);
	session.send_one(
		json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "git_checkout", "arguments": { "branch": "feature" } } }),
	);
	std::fs::write(dir.path().join("feature.txt"), "x\n").unwrap();
	session.send_one(
		json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "git_add", "arguments": { "files": ["."] } } }),
	);
	session.send_one(json!({
		"jsonrpc": "2.0", "id": 4, "method": "tools/call",
		"params": { "name": "git_commit", "arguments": {
			"message": "feature work", "author_name": "T", "author_email": "t@example.com"
		}}
	}));
	session.send_one(
		json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "git_checkout", "arguments": { "branch": "master" } } }),
	);

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": { "name": "git_merge", "arguments": { "branch": "feature" } } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
	assert!(
		text.contains("fast-forward") || text.contains("up-to-date"),
		"unexpected merge result: {text}"
	);

	// Merging again should now report "already up-to-date".
	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": { "name": "git_merge", "arguments": { "branch": "feature" } } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
	assert!(text.contains("up-to-date"));
}

#[test]
fn test_git_diff_unstaged_and_staged() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	// After two commits, there should be no unstaged or staged changes
	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_diff_unstaged", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("no unstaged changes"), "got: {text}");

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "git_diff_staged", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("no staged changes"), "got: {text}");

	// Stage a new file — now staged shows diff, unstaged shows nothing
	std::fs::write(dir.path().join("staged.txt"), "staged content\n").unwrap();
	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "git_add", "arguments": { "files": ["staged.txt"] } } }),
	);
	assert_eq!(resp["result"]["content"][0]["text"], json!("staged"));

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "git_diff_staged", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(
		!text.contains("no staged changes"),
		"expected staged diff, got: {text}"
	);
	assert!(
		text.contains("staged.txt") || text.contains("staged content"),
		"expected staged.txt in staged diff"
	);

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "git_diff_unstaged", "arguments": {} } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(
		text.contains("no unstaged changes"),
		"expected no unstaged changes, got: {text}"
	);
}

#[test]
fn test_git_diff_target() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	// repo has two commits: "Initial commit" and "Second commit".
	// diff HEAD~1 vs HEAD should show the difference between them.
	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_diff", "arguments": { "target": "HEAD~1" } } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(
		text.contains("+world"),
		"expected diff content, got: {text}"
	);

	// Flag injection defense
	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "git_diff", "arguments": { "target": "--help" } } }),
	);
	assert!(
		resp["error"].is_object(),
		"expected error for flag injection"
	);

	// Missing required target
	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "git_diff", "arguments": {} } }),
	);
	assert!(
		resp["error"].is_object(),
		"expected error for missing target"
	);
}

#[test]
fn test_git_branches_filtered() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	session.send_one(
		json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "git_branch_create", "arguments": { "name": "feature-a" } } }),
	);
	session.send_one(
		json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "git_branch_create", "arguments": { "name": "feature-b" } } }),
	);

	let resp = session.send_one(
		json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "git_branches", "arguments": { "branch_type": "local" } } }),
	);
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(
		text.contains("feature-a"),
		"expected feature-a in branches, got: {text}"
	);
	assert!(
		text.contains("feature-b"),
		"expected feature-b in branches, got: {text}"
	);
	assert!(
		text.contains("master"),
		"expected master in branches, got: {text}"
	);
}

#[test]
fn test_full_mcp_session() {
	let dir = setup_repo();
	let mut session = TestSession::spawn(dir.path());

	let init = session.send_one(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }));
	assert!(init["result"]["serverInfo"]["name"] == json!("gitforge"));

	let tools = session.send_one(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
	let tool_names: Vec<String> = tools["result"]["tools"]
		.as_array()
		.unwrap()
		.iter()
		.map(|t| t["name"].as_str().unwrap().to_string())
		.collect();

	let mut next_id = 3;
	for name in &tool_names {
		let args = match name.as_str() {
			"git_commit" => {
				json!({ "message": "m", "author_name": "a", "author_email": "a@b.com" })
			}
			"git_add" => json!({ "files": ["."] }),
			"git_branch_create" => json!({ "name": format!("branch-{next_id}") }),
			"git_checkout" => json!({ "branch": "master" }),
			"git_merge" => json!({ "branch": "master" }),
			"git_show" => json!({ "revision": "HEAD" }),
			"git_diff" => json!({ "target": "HEAD" }),
			"git_branches" => json!({ "branch_type": "local" }),
			_ => json!({}),
		};
		let resp = session.send_one(
			json!({ "jsonrpc": "2.0", "id": next_id, "method": "tools/call", "params": { "name": name, "arguments": args } }),
		);
		assert!(
			resp["result"].is_object() || resp["error"].is_object(),
			"tool '{name}' produced neither result nor error"
		);
		next_id += 1;
	}

	let resources =
		session.send_one(json!({ "jsonrpc": "2.0", "id": next_id, "method": "resources/list" }));
	for r in resources["result"]["resources"].as_array().unwrap() {
		next_id += 1;
		let uri = r["uri"].as_str().unwrap();
		let resp = session.send_one(
			json!({ "jsonrpc": "2.0", "id": next_id, "method": "resources/read", "params": { "uri": uri } }),
		);
		assert!(resp["result"].is_object());
	}

	// Notification: no response expected.
	let responses =
		session.send(&[json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })]);
	assert!(responses.is_empty());
}
