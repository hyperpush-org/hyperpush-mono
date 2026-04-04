//! End-to-end JSON-RPC tests for `meshc lsp` against backend-shaped files and
//! manifest-driven override-entry projects.
//!
//! Proves that a real LSP process behaves correctly on:
//! - the retained reference backend transport rail
//! - override-entry projects rooted by `mesh.toml` + `lib/start.mpl`
//! - clean and broken diagnostics publication over stdio JSON-RPC
//! - semantic provider requests that cross the project module graph
//! - document formatting through the shared formatter path
//! - signature help as an editor assist surface

mod support;

use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use support::m046_route_free as route_free;
use support::m051_reference_backend as retained_backend;

const MESSAGE_TIMEOUT: Duration = Duration::from_secs(8);
const DEFAULT_ENTRYPOINT: &str = "main.mpl";

fn repo_root() -> PathBuf {
    fs::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("meshc crate should live under compiler/")
            .parent()
            .expect("workspace root should be above compiler/"),
    )
    .expect("workspace root should canonicalize")
}

fn meshc_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_meshc"))
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m048-s02-lsp", test_name)
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "File '{}' is missing a parent directory",
            path.display()
        ));
    };
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create '{}': {error}", parent.display()))?;
    fs::write(path, contents)
        .map_err(|error| format!("Failed to write '{}': {error}", path.display()))
}

fn validate_relative_fixture_path(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(format!(
            "Fixture path '{}' must stay relative to the temp project root",
            path.display()
        ));
    }

    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Fixture path '{}' escapes the temp project root",
                    path.display()
                ));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

fn package_manifest(name: &str, entrypoint: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nentrypoint = \"{entrypoint}\"\n")
}

fn source_position(source: &str, needle: &str, occurrence: usize) -> (u64, u64) {
    let (byte_index, _) = source
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| {
            panic!(
                "could not find occurrence {} of {:?} in source",
                occurrence, needle
            )
        });
    let prefix = &source[..byte_index];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u64;
    let character = prefix
        .rsplit('\n')
        .next()
        .map(|segment| segment.chars().count())
        .unwrap_or_else(|| prefix.chars().count()) as u64;
    (line, character)
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[derive(Debug)]
struct OverrideProjectFixture {
    _tempdir: tempfile::TempDir,
    project_dir: PathBuf,
    entry_path: PathBuf,
    entry_uri: String,
    entry_source: String,
    nested_support_path: PathBuf,
    nested_support_uri: String,
    nested_support_source: String,
    artifacts: PathBuf,
}

struct OverrideProjectSpec<'a> {
    name: &'a str,
    manifest_entrypoint: &'a str,
    entry_path: &'a str,
    nested_support_path: &'a str,
    files: &'a [(&'a str, &'a str)],
}

fn write_override_project_fixture(
    test_name: &str,
    spec: &OverrideProjectSpec<'_>,
) -> Result<OverrideProjectFixture, String> {
    validate_relative_fixture_path(spec.manifest_entrypoint)?;
    validate_relative_fixture_path(spec.entry_path)?;
    validate_relative_fixture_path(spec.nested_support_path)?;

    let artifacts = artifact_dir(test_name);
    let tempdir =
        tempfile::tempdir().map_err(|error| format!("Failed to create temp dir: {error}"))?;
    let project_dir = tempdir.path().join(spec.name);
    fs::create_dir_all(&project_dir)
        .map_err(|error| format!("Failed to create '{}': {error}", project_dir.display()))?;

    write_file(
        &project_dir.join("mesh.toml"),
        &package_manifest(spec.name, spec.manifest_entrypoint),
    )?;

    for (relative_path, contents) in spec.files {
        validate_relative_fixture_path(relative_path)?;
        write_file(&project_dir.join(relative_path), contents)?;
    }

    let expected_entry = project_dir.join(spec.entry_path);
    if !expected_entry.is_file() {
        return Err(format!(
            "Fixture '{}' is missing expected executable entry '{}'; refusing to run a malformed acceptance project.",
            spec.name, spec.entry_path
        ));
    }

    let expected_nested_support = project_dir.join(spec.nested_support_path);
    if !expected_nested_support.is_file() {
        return Err(format!(
            "Fixture '{}' is missing nested support module '{}'; refusing to run a malformed acceptance project.",
            spec.name, spec.nested_support_path
        ));
    }

    route_free::archive_directory_tree(&project_dir, &artifacts.join("project"));

    let project_dir = fs::canonicalize(&project_dir).map_err(|error| {
        format!(
            "Failed to canonicalize '{}': {error}",
            project_dir.display()
        )
    })?;
    let entry_path = fs::canonicalize(&expected_entry).map_err(|error| {
        format!(
            "Failed to canonicalize '{}': {error}",
            expected_entry.display()
        )
    })?;
    let nested_support_path = fs::canonicalize(&expected_nested_support).map_err(|error| {
        format!(
            "Failed to canonicalize '{}': {error}",
            expected_nested_support.display()
        )
    })?;
    let entry_source = fs::read_to_string(&entry_path)
        .map_err(|error| format!("Failed to read '{}': {error}", entry_path.display()))?;
    let nested_support_source = fs::read_to_string(&nested_support_path).map_err(|error| {
        format!(
            "Failed to read '{}': {error}",
            nested_support_path.display()
        )
    })?;

    route_free::write_artifact(
        &artifacts.join("fixture-paths.txt"),
        format!(
            "project_dir: {}\nentry_path: {}\nnested_support_path: {}\n",
            project_dir.display(),
            entry_path.display(),
            nested_support_path.display()
        ),
    );

    Ok(OverrideProjectFixture {
        _tempdir: tempdir,
        project_dir,
        entry_uri: file_uri(&entry_path),
        entry_path,
        entry_source,
        nested_support_uri: file_uri(&nested_support_path),
        nested_support_path,
        nested_support_source,
        artifacts,
    })
}

fn override_only_nested_fixture(test_name: &str) -> Result<OverrideProjectFixture, String> {
    let spec = OverrideProjectSpec {
        name: "override-entry-lsp-project",
        manifest_entrypoint: "lib/start.mpl",
        entry_path: "lib/start.mpl",
        nested_support_path: "lib/support/message.mpl",
        files: &[
            (
                "lib/start.mpl",
                "from Lib.Support.Message import message\n\nfn main() do\n  let rendered = message()\n  println(\"proof=#{rendered}\")\nend\n",
            ),
            (
                "lib/support/message.mpl",
                "pub fn message() -> String do\n  \"nested-support\"\nend\n",
            ),
        ],
    };

    write_override_project_fixture(test_name, &spec)
}

fn override_precedence_nested_fixture(test_name: &str) -> Result<OverrideProjectFixture, String> {
    let spec = OverrideProjectSpec {
        name: "override-precedence-lsp-project",
        manifest_entrypoint: "lib/start.mpl",
        entry_path: "lib/start.mpl",
        nested_support_path: "lib/support/message.mpl",
        files: &[
            (
                DEFAULT_ENTRYPOINT,
                "fn main() do\n  println(\"legacy-root-main\")\nend\n",
            ),
            (
                "lib/start.mpl",
                "from Lib.Support.Message import message\n\nfn main() do\n  println(message())\nend\n",
            ),
            (
                "lib/support/message.mpl",
                "pub fn message() -> String do\n  \"override-precedence\"\nend\n",
            ),
        ],
    };

    write_override_project_fixture(test_name, &spec)
}

fn read_json_rpc_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read JSON-RPC header: {error}"))?;
        if bytes == 0 {
            return Ok(None);
        }
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            let parsed = value.trim().parse::<usize>().map_err(|error| {
                format!("invalid JSON-RPC Content-Length header {line:?}: {error}")
            })?;
            content_length = Some(parsed);
        }
    }

    let len = content_length.ok_or_else(|| "missing JSON-RPC Content-Length header".to_string())?;
    let mut body = vec![0; len];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("failed to read JSON-RPC body (len={len}): {error}"))?;
    serde_json::from_slice(&body).map(Some).map_err(|error| {
        format!(
            "failed to parse JSON-RPC body as JSON: {error}; raw body: {}",
            String::from_utf8_lossy(&body)
        )
    })
}

enum LspEvent {
    Message(Value),
    ReadError(String),
}

struct LspSession {
    child: Child,
    stdin: ChildStdin,
    rx: mpsc::Receiver<LspEvent>,
    pending: VecDeque<Value>,
    stderr: Arc<Mutex<String>>,
    trace: Arc<Mutex<Vec<String>>>,
    artifacts: PathBuf,
    session_label: String,
    next_id: u64,
}

impl LspSession {
    fn new(cwd: &Path, artifacts: PathBuf, session_label: &str) -> Self {
        let mut child = Command::new(meshc_bin())
            .current_dir(cwd)
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| {
                panic!(
                    "failed to spawn meshc lsp in {} (artifacts: {}): {error}",
                    cwd.display(),
                    artifacts.display()
                )
            });

        let stdin = child.stdin.take().expect("meshc lsp stdin should be piped");
        let stdout = child
            .stdout
            .take()
            .expect("meshc lsp stdout should be piped");
        let stderr_reader = child
            .stderr
            .take()
            .expect("meshc lsp stderr should be piped");

        let trace = Arc::new(Mutex::new(Vec::new()));
        let stdout_trace = Arc::clone(&trace);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_json_rpc_message(&mut reader) {
                    Ok(Some(message)) => {
                        stdout_trace
                            .lock()
                            .unwrap()
                            .push(format!("<<< {}", pretty_json(&message)));
                        if tx.send(LspEvent::Message(message)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        stdout_trace
                            .lock()
                            .unwrap()
                            .push(format!("!!! stdout read error: {error}"));
                        let _ = tx.send(LspEvent::ReadError(error));
                        break;
                    }
                }
            }
        });

        let stderr = Arc::new(Mutex::new(String::new()));
        let stderr_capture = Arc::clone(&stderr);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr_reader);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => stderr_capture.lock().unwrap().push_str(&line),
                    Err(error) => {
                        stderr_capture
                            .lock()
                            .unwrap()
                            .push_str(&format!("[stderr read error] {error}\n"));
                        break;
                    }
                }
            }
        });

        Self {
            child,
            stdin,
            rx,
            pending: VecDeque::new(),
            stderr,
            trace,
            artifacts,
            session_label: session_label.to_string(),
            next_id: 1,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));

        self.recv_response(id, method)
    }

    fn request_without_params(&mut self, method: &str) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        }));

        self.recv_response(id, method)
    }

    fn recv_response(&mut self, id: u64, method: &str) -> Value {
        let response = self.recv_matching(&format!("response for {method}"), |message| {
            message.get("id").and_then(Value::as_u64) == Some(id)
        });

        if let Some(error) = response.get("error") {
            self.persist_observability();
            panic!(
                "{method} returned JSON-RPC error: {}\n{}",
                error,
                self.failure_context(&format!("response for {method}"))
            );
        }

        response
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
    }

    fn send(&mut self, message: &Value) {
        self.trace
            .lock()
            .unwrap()
            .push(format!(">>> {}", pretty_json(message)));

        let body = serde_json::to_vec(message).expect("JSON-RPC payload should serialize");
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.stdin
            .write_all(header.as_bytes())
            .expect("failed to write JSON-RPC header");
        self.stdin
            .write_all(&body)
            .expect("failed to write JSON-RPC body");
        self.stdin.flush().expect("failed to flush JSON-RPC body");
    }

    fn recv_matching<F>(&mut self, label: &str, mut predicate: F) -> Value
    where
        F: FnMut(&Value) -> bool,
    {
        if let Some(index) = self.pending.iter().position(|message| predicate(message)) {
            return self
                .pending
                .remove(index)
                .expect("pending index should exist");
        }

        let deadline = Instant::now() + MESSAGE_TIMEOUT;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_else(|| Duration::from_millis(0));
            if remaining.is_zero() {
                self.persist_observability();
                panic!(
                    "timed out waiting for {label}\n{}",
                    self.failure_context(label)
                );
            }

            match self.rx.recv_timeout(remaining) {
                Ok(LspEvent::Message(message)) => {
                    if predicate(&message) {
                        return message;
                    }
                    self.pending.push_back(message);
                }
                Ok(LspEvent::ReadError(error)) => {
                    self.persist_observability();
                    panic!(
                        "meshc lsp emitted malformed JSON-RPC while waiting for {label}: {error}\n{}",
                        self.failure_context(label)
                    );
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.persist_observability();
                    panic!(
                        "timed out waiting for {label}\n{}",
                        self.failure_context(label)
                    );
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.persist_observability();
                    panic!(
                        "meshc lsp stdout disconnected while waiting for {label}\n{}",
                        self.failure_context(label)
                    );
                }
            }
        }
    }

    fn wait_for_diagnostics(&mut self, uri: &str, phase: &str) -> Vec<Value> {
        let label = format!("publishDiagnostics for {uri} during {phase}");
        let message = self.recv_matching(&label, |value| {
            value.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
                && value
                    .get("params")
                    .and_then(|params| params.get("uri"))
                    .and_then(Value::as_str)
                    == Some(uri)
        });

        let diagnostics = if let Some(diagnostics) = message
            .get("params")
            .and_then(|params| params.get("diagnostics"))
            .and_then(Value::as_array)
            .cloned()
        {
            diagnostics
        } else {
            self.persist_observability();
            panic!(
                "publishDiagnostics for {uri} during {phase} did not include a diagnostics array; raw message: {}\n{}",
                pretty_json(&message),
                self.failure_context(&label)
            );
        };

        println!(
            "[e2e_lsp] phase={phase} uri={uri} diagnostics={}",
            diagnostics.len()
        );
        diagnostics
    }

    fn stderr_output(&self) -> String {
        self.stderr.lock().unwrap().clone()
    }

    fn trace_output(&self) -> String {
        let trace = self.trace.lock().unwrap();
        if trace.is_empty() {
            "<empty trace>".to_string()
        } else {
            trace.join("\n")
        }
    }

    fn child_status_line(&mut self) -> String {
        match self.child.try_wait() {
            Ok(Some(status)) => format!("child_status: {:?}", status.code()),
            Ok(None) => "child_status: still running".to_string(),
            Err(error) => format!("child_status: unavailable ({error})"),
        }
    }

    fn persist_observability(&self) {
        route_free::write_artifact(
            &self
                .artifacts
                .join(format!("{}.trace.log", self.session_label)),
            self.trace_output(),
        );
        route_free::write_artifact(
            &self
                .artifacts
                .join(format!("{}.stderr.log", self.session_label)),
            self.stderr_output(),
        );
    }

    fn failure_context(&mut self, label: &str) -> String {
        let artifacts = self.artifacts.display().to_string();
        let child_status = self.child_status_line();
        let pending = format!("{:?}", self.pending);
        let stderr = self.stderr_output();
        let trace = self.trace_output();

        format!(
            "phase: {label}\nartifacts: {artifacts}\n{child_status}\npending messages: {pending}\nstderr:\n{stderr}\ntrace:\n{trace}"
        )
    }
}

impl Drop for LspSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.persist_observability();
    }
}

fn require_single_location<'a>(response: &'a Value, context: &str) -> &'a Value {
    let Some(result) = response.get("result") else {
        panic!(
            "{context} missing result payload: {}",
            pretty_json(response)
        );
    };

    match result {
        Value::Object(_) => result,
        Value::Array(locations) if locations.len() == 1 => &locations[0],
        Value::Array(locations) => panic!(
            "{context} expected exactly one location, got {} locations: {}",
            locations.len(),
            pretty_json(response)
        ),
        Value::Null => panic!(
            "{context} returned null instead of a definition location: {}",
            pretty_json(response)
        ),
        _ => panic!(
            "{context} returned malformed location payload: {}",
            pretty_json(response)
        ),
    }
}

#[test]
fn override_entry_fixture_rejects_missing_configured_entry_file() {
    let spec = OverrideProjectSpec {
        name: "broken-override-entry-lsp-project",
        manifest_entrypoint: "lib/start.mpl",
        entry_path: "lib/start.mpl",
        nested_support_path: "lib/support/message.mpl",
        files: &[(
            "lib/support/message.mpl",
            "pub fn message() -> String do\n  \"nested-support\"\nend\n",
        )],
    };

    let error = write_override_project_fixture("fixture-missing-entry", &spec)
        .expect_err("fixture writer should reject a missing override entry file");

    assert!(error.contains("lib/start.mpl"), "unexpected error: {error}");
    assert!(
        error.contains("malformed acceptance project"),
        "unexpected error: {error}"
    );
}

#[test]
fn override_entry_fixture_rejects_invalid_relative_paths() {
    let spec = OverrideProjectSpec {
        name: "invalid-relative-path-lsp-project",
        manifest_entrypoint: "lib/start.mpl",
        entry_path: "lib/start.mpl",
        nested_support_path: "../escape/support.mpl",
        files: &[(
            "lib/start.mpl",
            "fn main() do\n  println(\"invalid\")\nend\n",
        )],
    };

    let error = write_override_project_fixture("fixture-invalid-relative-path", &spec)
        .expect_err("fixture writer should reject paths that escape the temp project root");

    assert!(error.contains("escapes the temp project root"), "{error}");
}

#[test]
fn override_entry_fixture_supports_override_precedence_layout() {
    let fixture = override_precedence_nested_fixture("fixture-override-precedence")
        .expect("override-precedence fixture should materialize cleanly");

    assert!(
        fixture.project_dir.join(DEFAULT_ENTRYPOINT).is_file(),
        "override-precedence fixture should keep a root main.mpl for the precedence boundary"
    );
    assert!(
        fixture.entry_path.is_file(),
        "override-precedence fixture should keep the manifest-selected entry file"
    );
    assert!(
        fixture.nested_support_path.is_file(),
        "override-precedence fixture should keep the nested support module"
    );
}

#[test]
fn read_json_rpc_message_rejects_malformed_payload() {
    let body = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":";
    let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let mut cursor = std::io::Cursor::new(framed.into_bytes());

    let error = read_json_rpc_message(&mut cursor)
        .expect_err("malformed JSON-RPC payloads should fail instead of being treated as EOF");

    assert!(
        error.contains("failed to parse JSON-RPC body as JSON"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn lsp_json_rpc_reference_backend_flow() {
    let repo_root = repo_root();
    let artifacts = artifact_dir("reference-backend-flow");
    let reference_backend = retained_backend::retained_fixture_root();
    let health_path = fs::canonicalize(retained_backend::retained_health_path())
        .expect("retained reference-backend health file should exist");
    let jobs_path = fs::canonicalize(retained_backend::retained_jobs_path())
        .expect("retained reference-backend jobs file should exist");
    let health_uri = file_uri(&health_path);
    let jobs_uri = file_uri(&jobs_path);
    let health_source = fs::read_to_string(&health_path).expect("health source should be readable");
    let jobs_source = fs::read_to_string(&jobs_path).expect("jobs source should be readable");

    route_free::write_artifact(
        &artifacts.join("reference-paths.txt"),
        format!(
            "repo_root: {}\nfixture_root: {}\nhealth_path: {}\njobs_path: {}\n",
            repo_root.display(),
            reference_backend.display(),
            health_path.display(),
            jobs_path.display()
        ),
    );

    let mut session = LspSession::new(
        &reference_backend,
        artifacts.clone(),
        "retained-reference-backend",
    );

    let initialize = session.request(
        "initialize",
        json!({
            "processId": Value::Null,
            "rootUri": file_uri(&reference_backend),
            "capabilities": {},
        }),
    );
    assert_eq!(
        initialize["result"]["capabilities"]["documentFormattingProvider"].as_bool(),
        Some(true),
        "initialize should advertise document formatting support: {initialize:?}"
    );
    assert_eq!(
        initialize["result"]["capabilities"]["hoverProvider"].as_bool(),
        Some(true),
        "initialize should advertise hover support: {initialize:?}"
    );

    session.notify("initialized", json!({}));

    println!("[e2e_lsp] phase=reference-backend didOpen uri={health_uri}");
    session.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": health_uri,
                "languageId": "mesh",
                "version": 1,
                "text": health_source,
            }
        }),
    );
    let health_open_diagnostics = session.wait_for_diagnostics(&health_uri, "health didOpen");
    assert!(
        health_open_diagnostics.is_empty(),
        "{} should open cleanly, got diagnostics: {:?}\nartifacts: {}",
        retained_backend::RETAINED_FIXTURE_HEALTH_RELATIVE,
        health_open_diagnostics,
        artifacts.display()
    );

    println!("[e2e_lsp] phase=reference-backend didOpen uri={jobs_uri}");
    session.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": jobs_uri,
                "languageId": "mesh",
                "version": 1,
                "text": jobs_source,
            }
        }),
    );
    let jobs_open_diagnostics = session.wait_for_diagnostics(&jobs_uri, "jobs didOpen");
    assert!(
        jobs_open_diagnostics.is_empty(),
        "{} should open cleanly, got diagnostics: {:?}\nartifacts: {}",
        retained_backend::RETAINED_FIXTURE_JOBS_RELATIVE,
        jobs_open_diagnostics,
        artifacts.display()
    );

    let create_job_call = "create_job_response(job, body)";
    let (create_job_call_line, create_job_call_character) =
        source_position(&jobs_source, create_job_call, 0);
    let create_job_definition = "fn create_job_response(job :: Job, payload :: String) do";
    let (create_job_definition_line, _) = source_position(&jobs_source, create_job_definition, 0);
    let log_create_success_call = "log_create_success(job, payload)";
    let (log_create_success_line, log_create_success_character) =
        source_position(&jobs_source, log_create_success_call, 0);
    let log_create_success_active_param = log_create_success_character
        + log_create_success_call
            .find("payload")
            .expect("log_create_success call should mention payload") as u64;

    let hover = session.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": jobs_uri },
            "position": {
                "line": create_job_call_line,
                "character": create_job_call_character,
            },
        }),
    );
    let hover_contents = hover["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_default();
    assert!(
        hover_contents.contains("create_job_response")
            || hover_contents.contains("Job")
            || hover_contents.contains("String"),
        "hover should return function type information for backend code, got: {hover:?}"
    );

    let definition = session.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": jobs_uri },
            "position": {
                "line": create_job_call_line,
                "character": create_job_call_character,
            },
        }),
    );
    let definition_location = require_single_location(&definition, "retained reference-backend definition");
    println!(
        "[e2e_lsp] phase=reference-backend provider=definition uri={jobs_uri} response={} ",
        pretty_json(&definition)
    );
    assert_eq!(
        definition_location["uri"].as_str(),
        Some(jobs_uri.as_str()),
        "definition should stay within {} for create_job_response call: {definition:?}",
        retained_backend::RETAINED_FIXTURE_JOBS_RELATIVE,
    );
    assert_eq!(
        definition_location["range"]["start"]["line"].as_u64(),
        Some(create_job_definition_line),
        "definition should jump to create_job_response definition, got: {definition:?}"
    );

    let signature_help = session.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": jobs_uri },
            "position": {
                "line": log_create_success_line,
                "character": log_create_success_active_param,
            },
        }),
    );
    let signature_label = signature_help["result"]["signatures"][0]["label"]
        .as_str()
        .unwrap_or_default();
    assert!(
        signature_label.contains("log_create_success(job: Job, payload: String) -> ()"),
        "signature help should name the backend helper being called, got: {signature_help:?}"
    );
    assert_eq!(
        signature_help["result"]["activeParameter"].as_u64(),
        Some(1),
        "signature help should identify the second parameter inside log_create_success(...), got: {signature_help:?}"
    );

    let unformatted_health = health_source.replacen(
        "  let wrapped = if String.length(value) > 0 do\n",
        "let wrapped = if String.length(value) > 0 do\n",
        1,
    );
    assert_ne!(
        unformatted_health, health_source,
        "the health formatter probe must actually make the backend file non-canonical"
    );

    session.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": health_uri, "version": 2 },
            "contentChanges": [{ "text": unformatted_health }],
        }),
    );
    let health_change_diagnostics =
        session.wait_for_diagnostics(&health_uri, "health unformatted didChange");
    assert!(
        health_change_diagnostics.is_empty(),
        "unformatted backend text should still type-check cleanly before formatting, got diagnostics: {:?}\nartifacts: {}",
        health_change_diagnostics,
        artifacts.display()
    );

    let formatting = session.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": health_uri },
            "options": { "tabSize": 2, "insertSpaces": true },
        }),
    );
    let edits = formatting["result"]
        .as_array()
        .expect("formatting should return a text edit array for unformatted backend text");
    assert_eq!(
        edits.len(),
        1,
        "formatting should perform a single full-document replacement edit, got: {formatting:?}"
    );
    assert_eq!(
        edits[0]["newText"].as_str(),
        Some(health_source.as_str()),
        "formatting should restore canonical {} text",
        retained_backend::RETAINED_FIXTURE_HEALTH_RELATIVE,
    );

    let invalid_health = format!(
        "{}\nlet broken :: Int = \"oops\"\n",
        health_source.trim_end()
    );
    session.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": health_uri, "version": 3 },
            "contentChanges": [{ "text": invalid_health }],
        }),
    );
    let invalid_diagnostics = session.wait_for_diagnostics(&health_uri, "health invalid didChange");
    assert!(
        !invalid_diagnostics.is_empty(),
        "invalid backend-shaped text should publish diagnostics instead of staying green"
    );
    assert!(
        invalid_diagnostics.iter().any(|diag| {
            diag["message"]
                .as_str()
                .map(|message| message.contains("type mismatch") || message.contains("Parse error"))
                .unwrap_or(false)
        }),
        "invalid backend diagnostics should describe the backend buffer error instead of falling back to bogus import failures, got: {:?}",
        invalid_diagnostics
    );

    let shutdown = session.request_without_params("shutdown");
    assert!(
        shutdown.get("result").is_some(),
        "shutdown should return a JSON-RPC result, got: {shutdown:?}"
    );
}

#[test]
fn lsp_json_rpc_override_entry_flow() {
    let fixture = override_only_nested_fixture("override-entry-flow")
        .unwrap_or_else(|error| panic!("override-entry fixture should materialize: {error}"));
    let message_call = "message()";
    let (message_call_line, message_call_character) =
        source_position(&fixture.entry_source, message_call, 0);

    let mut session = LspSession::new(
        &fixture.project_dir,
        fixture.artifacts.clone(),
        "override-entry",
    );

    let initialize = session.request(
        "initialize",
        json!({
            "processId": Value::Null,
            "rootUri": file_uri(&fixture.project_dir),
            "capabilities": {},
        }),
    );
    assert_eq!(
        initialize["result"]["capabilities"]["hoverProvider"].as_bool(),
        Some(true),
        "initialize should advertise hover support for override-entry projects: {initialize:?}"
    );

    session.notify("initialized", json!({}));

    println!(
        "[e2e_lsp] phase=override-entry didOpen uri={} path={}",
        fixture.entry_uri,
        fixture.entry_path.display()
    );
    session.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": fixture.entry_uri,
                "languageId": "mesh",
                "version": 1,
                "text": fixture.entry_source,
            }
        }),
    );
    let entry_diagnostics =
        session.wait_for_diagnostics(&fixture.entry_uri, "override-entry didOpen entry");
    assert!(
        entry_diagnostics.is_empty(),
        "override-entry entry file should open cleanly through live JSON-RPC, got diagnostics: {:?}\nartifacts: {}",
        entry_diagnostics,
        fixture.artifacts.display()
    );

    println!(
        "[e2e_lsp] phase=override-entry didOpen uri={} path={}",
        fixture.nested_support_uri,
        fixture.nested_support_path.display()
    );
    session.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": fixture.nested_support_uri,
                "languageId": "mesh",
                "version": 1,
                "text": fixture.nested_support_source,
            }
        }),
    );
    let support_diagnostics = session.wait_for_diagnostics(
        &fixture.nested_support_uri,
        "override-entry didOpen nested support",
    );
    assert!(
        support_diagnostics.is_empty(),
        "override-entry nested support file should open cleanly through live JSON-RPC, got diagnostics: {:?}\nartifacts: {}",
        support_diagnostics,
        fixture.artifacts.display()
    );

    let hover = session.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": fixture.entry_uri },
            "position": {
                "line": message_call_line,
                "character": message_call_character,
            },
        }),
    );
    println!(
        "[e2e_lsp] phase=override-entry provider=hover uri={} response={}",
        fixture.entry_uri,
        pretty_json(&hover)
    );
    let hover_contents = hover["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_default();
    assert!(
        hover.get("result").is_some() && !hover_contents.is_empty(),
        "override-entry hover should return type information for the imported nested helper, got: {}\nartifacts: {}",
        pretty_json(&hover),
        fixture.artifacts.display()
    );
    assert!(
        hover_contents.contains("String"),
        "override-entry hover should prove nested import typing instead of falling back to empty local analysis, got: {}\nartifacts: {}",
        pretty_json(&hover),
        fixture.artifacts.display()
    );

    let shutdown = session.request_without_params("shutdown");
    assert!(
        shutdown.get("result").is_some(),
        "shutdown should return a JSON-RPC result for override-entry flow, got: {shutdown:?}"
    );
}
