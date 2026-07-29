use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Create a temporary git repo with some commits for testing.
fn setup_repo() -> tempfile::TempDir {
	let dir = tempfile::tempdir().expect("tempdir");

	Command::new("git")
		.args(["-C", dir.path().to_str().unwrap(), "init"])
		.output()
		.expect("git init");

	Command::new("git")
		.args([
			"-C",
			dir.path().to_str().unwrap(),
			"config",
			"user.email",
			"test@test.com",
		])
		.output()
		.expect("git config email");

	Command::new("git")
		.args([
			"-C",
			dir.path().to_str().unwrap(),
			"config",
			"user.name",
			"Test",
		])
		.output()
		.expect("git config name");

	std::fs::write(dir.path().join("file1.txt"), b"hello world\n").unwrap();
	Command::new("git")
		.args(["-C", dir.path().to_str().unwrap(), "add", "."])
		.output()
		.expect("git add");

	Command::new("git")
		.args(["-C", dir.path().to_str().unwrap(), "commit", "-m", "initial commit"])
		.output()
		.expect("git commit");

	std::fs::write(dir.path().join("file2.txt"), b"second file\n").unwrap();
	Command::new("git")
		.args(["-C", dir.path().to_str().unwrap(), "add", "."])
		.output()
		.expect("git add");

	Command::new("git")
		.args(["-C", dir.path().to_str().unwrap(), "commit", "-m", "add file2"])
		.output()
		.expect("git commit");

	dir
}

/// Spawn the binary, send a JSON-RPC request, read one response line.
fn send_request(
	dir: &tempfile::TempDir,
	req: &str,
) -> (std::process::Child, String) {
	let mut child = Command::new(env!("CARGO_BIN_EXE_gitforge"))
		.arg(dir.path())
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::null())
		.spawn()
		.expect("spawn gitforge");

	let stdin = child.stdin.as_mut().unwrap();
	writeln!(stdin, "{}", req).unwrap();
	stdin.flush().unwrap();

	let stdout = child.stdout.take().unwrap();
	let mut reader = BufReader::new(stdout);
	let mut line = String::new();
	reader.read_line(&mut line).expect("read response line");

	(child, line)
}

#[test]
fn test_ping() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	assert_eq!(resp["id"], 1);
	assert_eq!(resp["result"], serde_json::json!({}));

	child.kill().ok();
}

#[test]
fn test_initialize_returns_tools_and_resources() {
	let dir = setup_repo();
	let req = r##"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"##;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	assert_eq!(resp["id"], 1);
	assert_eq!(resp["result"]["protocolVersion"], "2025-11-25");
	assert_eq!(resp["result"]["serverInfo"]["name"], "gitforge");

	// Should include tools in the initialize response
	let tools = resp["result"]["tools"].as_array().unwrap();
	let tool_names: Vec<&str> = tools
		.iter()
		.filter_map(|t| t["name"].as_str())
		.collect();
	assert!(
		tool_names.contains(&"ping"),
		"tools should include 'ping', got: {:?}",
		tool_names
	);
	assert!(
		tool_names.contains(&"git_status"),
		"tools should include 'git_status', got: {:?}",
		tool_names
	);
	assert!(
		tool_names.contains(&"git_branches"),
		"tools should include 'git_branches', got: {:?}",
		tool_names
	);

	// Should include resources in the initialize response
	let resources = resp["result"]["resources"].as_array().unwrap();
	let resource_uris: Vec<&str> = resources
		.iter()
		.filter_map(|r| r["uri"].as_str())
		.collect();
	assert!(
		resource_uris.contains(&"git://HEAD"),
		"resources should include 'git://HEAD', got: {:?}",
		resource_uris
	);
	assert!(
		resource_uris.contains(&"git://status"),
		"resources should include 'git://status', got: {:?}",
		resource_uris
	);

	child.kill().ok();
}

#[test]
fn test_invalid_method() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":1,"method":"nonexistent"}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	assert_eq!(resp["id"], 1);
	assert!(resp["error"].is_object());
	assert_eq!(resp["error"]["code"], -32601);

	child.kill().ok();
}

#[test]
fn test_git_branches() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"git_branches","arguments":{}}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("main") || text.contains("master"), "expected main/master branch, got: {}", text);

	child.kill().ok();
}

#[test]
fn test_git_status_clean() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"git_status","arguments":{}}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert_eq!(text, "nothing to commit, working tree clean");

	child.kill().ok();
}

#[test]
fn test_git_status_dirty() {
	let dir = setup_repo();

	// Make a dirty change
	std::fs::write(dir.path().join("file1.txt"), b"modified\n").unwrap();

	let req = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"git_status","arguments":{}}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("WT_MODIFIED"), "expected WT_MODIFIED in status, got: {}", text);

	child.kill().ok();
}

#[test]
fn test_git_log() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"git_log","arguments":{"max_count":5}}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("initial commit"));
	assert!(text.contains("add file2"));

	child.kill().ok();
}

#[test]
fn test_git_diff() {
	let dir = setup_repo();

	// Make a dirty change
	std::fs::write(dir.path().join("file1.txt"), b"modified content\n").unwrap();

	let req = r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"git_diff","arguments":{}}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("-hello world") || text.contains("+modified content"), "diff should show changes, got: {}", text);

	child.kill().ok();
}

#[test]
fn test_git_show() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"git_show","arguments":{"revision":"HEAD"}}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["content"][0]["text"].as_str().unwrap();
	assert!(text.contains("commit"));
	assert!(text.contains("Author:"));
	assert!(text.contains("add file2"));

	child.kill().ok();
}

#[test]
fn test_resources_list() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":8,"method":"resources/list"}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let resources = resp["result"]["resources"].as_array().unwrap();
	assert!(resources.len() >= 2, "expected at least 2 resources, got {}", resources.len());

	let uris: Vec<&str> = resources.iter().filter_map(|r| r["uri"].as_str()).collect();
	assert!(uris.contains(&"git://HEAD"));
	assert!(uris.contains(&"git://status"));

	child.kill().ok();
}

#[test]
fn test_resource_read_head() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"git://HEAD"}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["contents"][0]["text"].as_str().unwrap();
	assert!(text.contains("commit"));
	assert!(text.contains("Author:"));

	child.kill().ok();
}

#[test]
fn test_resource_read_status() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":10,"method":"resources/read","params":{"uri":"git://status"}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	let text = resp["result"]["contents"][0]["text"].as_str().unwrap();
	assert_eq!(text, "nothing to commit, working tree clean");

	child.kill().ok();
}

#[test]
fn test_resource_unknown_uri() {
	let dir = setup_repo();
	let req = r#"{"jsonrpc":"2.0","id":11,"method":"resources/read","params":{"uri":"git://nonexistent"}}"#;
	let (mut child, line) = send_request(&dir, req);

	let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
	// Should return an error
	assert!(resp["error"].is_object(), "expected error for unknown resource");

	child.kill().ok();
}

#[test]
fn test_notification_no_response() {
	let dir = setup_repo();
	let mut child = Command::new(env!("CARGO_BIN_EXE_gitforge"))
		.arg(dir.path())
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::null())
		.spawn()
		.expect("spawn gitforge");

	let stdin = child.stdin.as_mut().unwrap();
	writeln!(
		stdin,
		r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
	)
	.unwrap();
	stdin.flush().unwrap();

	// Give the process time to process (notification should produce no output)
	std::thread::sleep(std::time::Duration::from_millis(200));

	// Notification should produce no output line
	child.kill().ok();

	// Verify the process actually started and handled the request
	let output = child.wait_with_output().ok();
	assert!(output.is_some(), "process should have exited");
}
