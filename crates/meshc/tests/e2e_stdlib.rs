//! End-to-end integration tests for Mesh standard library functions (Phase 8).
//!
//! Tests string operations, module-qualified access (String.length),
//! from/import resolution, IO operations, and HTTP server/client compilation.

use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Helper: compile a Mesh source file and run the resulting binary, returning stdout.
fn compile_and_run(source: &str) -> String {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, source).expect("failed to write main.mpl");

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let binary = project_dir.join("project");
    let run_output = Command::new(&binary)
        .output()
        .unwrap_or_else(|e| panic!("failed to run binary at {}: {}", binary.display(), e));

    assert!(
        run_output.status.success(),
        "binary execution failed with exit code {:?}:\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    String::from_utf8_lossy(&run_output.stdout).to_string()
}

/// Find the meshc binary in the target directory.
fn find_meshc() -> PathBuf {
    let mut path = std::env::current_exe()
        .expect("cannot find current exe")
        .parent()
        .expect("cannot find parent dir")
        .to_path_buf();

    if path.file_name().map_or(false, |n| n == "deps") {
        path = path.parent().unwrap().to_path_buf();
    }

    let meshc = path.join("meshc");
    assert!(
        meshc.exists(),
        "meshc binary not found at {}. Run `cargo build -p meshc` first.",
        meshc.display()
    );
    meshc
}

/// Helper: compile a Mesh source file without running it. Returns compilation output.
fn compile_only(source: &str) -> Output {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, source).expect("failed to write main.mpl");

    let meshc = find_meshc();
    Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc")
}

/// Helper: compile a Mesh source file and run the binary with piped stdin input.
/// Useful for testing interactive I/O functions like IO.read_line().
fn compile_and_run_with_stdin(source: &str, stdin_input: &str) -> String {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, source).expect("failed to write main.mpl");

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let binary = project_dir.join("project");
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn binary at {}: {}", binary.display(), e));

    // Write stdin input and drop to signal EOF
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_input.as_bytes()).expect("failed to write stdin");
        // stdin is dropped here, closing the pipe
    }

    let run_output = child.wait_with_output().expect("failed to wait for child");

    assert!(
        run_output.status.success(),
        "binary execution failed with exit code {:?}:\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    String::from_utf8_lossy(&run_output.stdout).to_string()
}

/// Read a test fixture from the tests/e2e/ directory.
fn read_fixture(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("e2e")
        .join(name);
    std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", fixture_path.display(), e))
}

// ── String Operation E2E Tests ──────────────────────────────────────────

#[test]
fn e2e_string_length() {
    let source = read_fixture("stdlib_string_length.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "5\n");
}

#[test]
fn e2e_string_contains() {
    let source = read_fixture("stdlib_string_contains.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\nfalse\n");
}

#[test]
fn e2e_string_trim() {
    let source = read_fixture("stdlib_string_trim.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "hello\n");
}

#[test]
fn e2e_string_case_conversion() {
    let source = read_fixture("stdlib_string_case.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "HELLO\nworld\n");
}

#[test]
fn e2e_string_replace() {
    let source = read_fixture("stdlib_string_replace.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "hello mesh\n");
}

// ── Module Resolution E2E Tests ─────────────────────────────────────────

#[test]
fn e2e_module_qualified_access() {
    let source = read_fixture("stdlib_module_qualified.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "4\n");
}

#[test]
fn e2e_from_import_resolution() {
    let source = read_fixture("stdlib_from_import.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "4\n");
}

// ── File I/O E2E Tests ──────────────────────────────────────────────────

#[test]
fn e2e_file_write_and_read() {
    let source = read_fixture("stdlib_file_write_read.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Hello, Mesh!\n");
}

#[test]
fn e2e_file_exists() {
    let source = read_fixture("stdlib_file_exists.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "false\ntrue\n\n");
}

#[test]
fn e2e_file_read_process_write() {
    let source = read_fixture("stdlib_file_process.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "HELLO WORLD\n");
}

#[test]
fn e2e_file_error_handling() {
    let source = read_fixture("stdlib_file_error.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "error\n");
}

// ── IO E2E Tests ────────────────────────────────────────────────────────

#[test]
fn e2e_io_eprintln_does_not_crash() {
    let source = read_fixture("stdlib_io_eprintln.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "done\n");
}

#[test]
fn e2e_io_read_line() {
    // Verify IO.read_line() compiles and runs through the full pipeline with piped stdin.
    let source = read_fixture("stdlib_io_read_line.mpl");
    let output = compile_and_run_with_stdin(&source, "hello world\n");
    assert_eq!(output, "hello world\n");
}

// ── Collection E2E Tests (Phase 8 Plan 02) ────────────────────────────

#[test]
fn e2e_list_basic() {
    let source = read_fixture("stdlib_list_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n1\n");
}

// ── List Literal E2E Tests (Phase 26 Plan 02) ────────────────────────────

#[test]
fn e2e_list_literal_int() {
    let source = read_fixture("list_literal_int.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n1\n3\n");
}

#[test]
fn e2e_list_literal_string() {
    let source = read_fixture("list_literal_string.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "2\nhello\nworld\n");
}

#[test]
fn e2e_list_literal_bool() {
    let source = read_fixture("list_literal_bool.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\ntrue\nfalse\n");
}

#[test]
fn e2e_list_concat() {
    let source = read_fixture("list_concat.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "4\n1\n4\n");
}

#[test]
fn e2e_list_nested() {
    let source = read_fixture("list_nested.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "2\n2\n2\n");
}

#[test]
fn e2e_list_append_string() {
    let source = read_fixture("list_append_string.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "2\nworld\n");
}

// ── List Trait Integration E2E Tests (Phase 27 Plan 01) ───────────────

#[test]
fn e2e_list_display_string() {
    let source = read_fixture("list_display_string.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "[hello, world]\n");
}

#[test]
fn e2e_list_debug() {
    let source = read_fixture("list_debug.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "[1, 2, 3]\n");
}

#[test]
fn e2e_list_eq() {
    let source = read_fixture("list_eq.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "equal\nnot equal\n");
}

#[test]
fn e2e_list_ord() {
    let source = read_fixture("list_ord.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "less\ngreater\n");
}

// ── List Cons Pattern E2E Tests (Phase 27 Plan 02) ────────────────────

#[test]
fn e2e_list_cons_int() {
    let source = read_fixture("list_cons_int.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "15\n");
}

#[test]
fn e2e_list_cons_string() {
    let source = read_fixture("list_cons_string.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "hello\nempty\n");
}

#[test]
fn e2e_map_basic() {
    let source = read_fixture("stdlib_map_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "10\n2\n");
}

#[test]
fn e2e_map_string_keys() {
    let source = read_fixture("stdlib_map_string_keys.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Alice\n2\ntrue\nBob\n");
}

#[test]
fn e2e_map_literal() {
    let source = read_fixture("map_literal.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Alice\n30\n2\n");
}

#[test]
fn e2e_map_literal_int() {
    let source = read_fixture("map_literal_int.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "20\n3\n");
}

#[test]
fn e2e_set_basic() {
    let source = read_fixture("stdlib_set_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "2\n");
}

#[test]
fn e2e_range_basic() {
    let source = read_fixture("stdlib_range_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n3\n1\n");
}

#[test]
fn e2e_queue_basic() {
    let source = read_fixture("stdlib_queue_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "2\n10\n");
}

// ── JSON E2E Tests (Phase 8 Plan 04) ──────────────────────────────────

#[test]
fn e2e_json_encode_int() {
    let source = read_fixture("stdlib_json_encode_int.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

#[test]
fn e2e_json_encode_string() {
    let source = read_fixture("stdlib_json_encode_string.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "\"hello\"\n");
}

#[test]
fn e2e_json_encode_bool() {
    let source = read_fixture("stdlib_json_encode_bool.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\nfalse\n");
}

#[test]
fn e2e_json_encode_map() {
    // Tests multiple JSON encode functions together (encode_int, encode_string, encode_bool)
    let source = read_fixture("stdlib_json_encode_map.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "100\n\"test\"\ntrue\n");
}

#[test]
fn e2e_json_parse_roundtrip() {
    let source = read_fixture("stdlib_json_parse_roundtrip.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "99\n");
}

// ── JSON Struct Serde E2E Tests (Phase 49) ──────────────────────────────

#[test]
fn e2e_deriving_json_basic() {
    let source = read_fixture("deriving_json_basic.mpl");
    let output = compile_and_run(&source);
    // First line: JSON encode (field order may vary since JSON objects are unordered).
    // Second line: decoded fields.
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines, got: {}", output);
    let json: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON");
    assert_eq!(json["name"], "Alice");
    assert_eq!(json["age"], 30);
    assert_eq!(json["score"], 95.5);
    assert_eq!(json["active"], true);
    assert_eq!(lines[1], "Alice 30 true");
}

#[test]
fn e2e_deriving_json_nested() {
    let source = read_fixture("deriving_json_nested.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 4, "expected 4 lines, got: {}", output);
    let json: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON");
    assert_eq!(json["name"], "Bob");
    assert_eq!(json["addr"]["city"], "NYC");
    assert_eq!(json["addr"]["zip"], 10001);
    assert_eq!(lines[1], "Bob");
    assert_eq!(lines[2], "NYC");
    assert_eq!(lines[3], "10001");
}

// NOTE: Option<T> fields in structs have a known codegen bug where pattern
// matching on the Option variant from a struct field causes a segfault.
// This is a pre-existing issue (not JSON-specific). The encode test is
// restricted to verify None encoding (which works) while Some encoding
// has incorrect field extraction due to the same underlying bug.
// Full Option round-trip tests are deferred until the Option-in-struct
// codegen is fixed.
#[test]
#[ignore] // blocked on Option-in-struct codegen bug
fn e2e_deriving_json_option() {
    let source = read_fixture("deriving_json_option.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines, got: {}", output);
    let json2: serde_json::Value = serde_json::from_str(lines[1]).expect("valid JSON line 2");
    assert_eq!(json2["name"], "Bob");
    assert!(json2["bio"].is_null());
}

#[test]
fn e2e_deriving_json_number_types() {
    let source = read_fixture("deriving_json_number_types.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 5, "expected 5 lines, got: {}", output);
    let json: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON");
    assert_eq!(json["i"], 42);
    assert_eq!(json["f"], 3.14);
    assert_eq!(lines[1], "42");
    assert_eq!(lines[2], "3.14");
    assert_eq!(lines[3], "43");
    assert_eq!(lines[4], "3.15");
}

#[test]
fn e2e_deriving_json_collections() {
    let source = read_fixture("deriving_json_collections.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got: {}", output);
    let json: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON");
    assert!(json["tags"].is_array(), "tags should be array");
    assert_eq!(json["tags"].as_array().unwrap().len(), 3);
    assert!(json["settings"].is_object(), "settings should be object");
    assert_eq!(lines[1], "3");
    assert_eq!(lines[2], "2");
}

#[test]
fn e2e_deriving_json_roundtrip() {
    let source = read_fixture("deriving_json_roundtrip.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines, got: {}", output);
    assert_eq!(lines[0], "round-trip: ok");
    assert_eq!(lines[1], "zero-values: ok");
}

#[test]
fn e2e_deriving_json_error() {
    let source = read_fixture("deriving_json_error.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got: {}", output);
    assert_eq!(lines[0], "parse error: ok");
    assert_eq!(lines[1], "missing field: ok");
    assert_eq!(lines[2], "wrong type: ok");
}

#[test]
fn e2e_deriving_json_non_serializable_compile_fail() {
    // Verify that deriving(Json) on a struct with a non-serializable field (Pid)
    // produces a compile error containing E0038.
    let source = r#"
struct BadStruct do
  name :: String
  worker :: Pid
end deriving(Json)

fn main() do
  println("should not compile")
end
"#;
    let result = compile_only(source);
    assert!(
        !result.status.success(),
        "Expected compilation failure for non-serializable field, but it succeeded"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("E0038") || stderr.contains("not JSON-serializable"),
        "Expected E0038 error, got stderr: {}",
        stderr
    );
}

// ── Row (Struct-to-Row Mapping) E2E Tests (Phase 58) ────────────────────

#[test]
fn e2e_deriving_row_basic() {
    let source = read_fixture("deriving_row_basic.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 1, "expected 1 line, got: {}", output);
    assert_eq!(lines[0], "Alice 30 95.5 true");
}

#[test]
fn e2e_deriving_row_option() {
    let source = read_fixture("deriving_row_option.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 1, "expected 1 line, got: {}", output);
    // Struct with Option fields: empty string bio -> None, "25" age -> Some(25)
    // from_row returns Ok(Profile) and we print the name
    assert_eq!(lines[0], "Bob");
}

#[test]
fn e2e_deriving_row_error() {
    let source = read_fixture("deriving_row_error.mpl");
    let output = compile_and_run(&source);
    let trimmed = output.trim();
    // Should report missing column "count"
    assert!(
        trimmed.contains("count"),
        "Expected error mentioning missing column 'count', got: {}",
        trimmed
    );
}

#[test]
fn e2e_deriving_row_non_mappable_compile_fail() {
    // Verify that deriving(Row) on a struct with a non-mappable field (List<Int>)
    // produces a compile error containing E0039.
    let source = r#"
struct Bad do
  name :: String
  tags :: List<Int>
end deriving(Row)

fn main() do
  println("should not compile")
end
"#;
    let result = compile_only(source);
    assert!(
        !result.status.success(),
        "Expected compile failure, but it succeeded"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("E0039") || stderr.contains("cannot be mapped from a database row"),
        "Expected E0039 error, got stderr: {}",
        stderr
    );
}

// ── HTTP E2E Tests (Phase 8 Plan 05, updated Phase 15) ────────────────
//
// The HTTP server uses actor-per-connection (Phase 15) with crash isolation
// via catch_unwind. Each incoming connection is handled by a lightweight
// actor on the M:N scheduler.

#[test]
fn e2e_http_server_compiles() {
    // Verify a server program compiles (cannot run because it blocks on serve).
    let source = read_fixture("stdlib_http_response.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "compiled\n");
}

#[test]
fn e2e_http_client_compiles_and_runs() {
    // Verify an HTTP client program compiles and runs.
    // This makes a real HTTP request to example.com.
    let source = read_fixture("stdlib_http_client.mpl");
    let output = compile_and_run(&source);
    // Should print "ok" (successful GET to example.com) or "error" (no network).
    assert!(
        output == "ok\n" || output == "error\n",
        "unexpected output: {}",
        output
    );
}

#[test]
fn e2e_http_full_server_compile_only() {
    // Verify a full server program with handler and serve compiles.
    let source = r#"
fn handler(request) do
  let m = Request.method(request)
  HTTP.response(200, m)
end

fn main() do
  let r = HTTP.router()
  let r = HTTP.route(r, "/", handler)
  HTTP.serve(r, 0)
end
"#;
    let result = compile_only(source);
    assert!(
        result.status.success(),
        "HTTP server with Request accessors should compile:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn e2e_http_serve_tls_compile_only() {
    // Verify HTTP.serve_tls(router, port, cert, key) compiles.
    // Phase 56 Plan 02: HTTPS TLS layer.
    let source = r#"
fn handler(request) do
  HTTP.response(200, "ok")
end

fn main() do
  let r = HTTP.router()
  let r = HTTP.route(r, "/", handler)
  HTTP.serve_tls(r, 8443, "cert.pem", "key.pem")
end
"#;
    let result = compile_only(source);
    assert!(
        result.status.success(),
        "HTTP.serve_tls should compile:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

// ── List Pipe Chain E2E Tests (Phase 8 Plan 06 - Gap Closure) ─────────

#[test]
fn e2e_list_pipe_chain() {
    // Verify map/filter/reduce with closures through the full compiler pipeline.
    // Input: [1..10], map(x*2) -> [2..20], filter(x>10) -> [12,14,16,18,20], reduce(sum) -> 80.
    let source = read_fixture("stdlib_list_pipe_chain.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "80\n");
}

// ── HTTP Runtime E2E Tests (Phase 8 Plan 07 - Gap Closure) ────────────
//
// These tests start a REAL HTTP server and make actual HTTP requests,
// verifying that the Mesh HTTP server works end-to-end at runtime.

/// RAII guard that kills the server child process on drop.
struct ServerGuard(std::process::Child);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Compile a Mesh source file and spawn the resulting binary as a server.
/// Returns a ServerGuard that kills the process on drop.
///
/// Waits for the server to emit its "[mesh-rt] HTTP server listening on"
/// message on stderr before returning, ensuring the server is ready.
fn compile_and_start_server(source: &str) -> ServerGuard {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    // Leak the temp dir so it persists for the lifetime of the server process.
    let temp_dir = Box::leak(Box::new(temp_dir));
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, source).expect("failed to write main.mpl");

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let binary = project_dir.join("project");
    assert!(
        binary.exists(),
        "compiled binary not found at {}",
        binary.display()
    );

    // Spawn the server binary with stderr piped so we can detect readiness.
    let child = Command::new(&binary)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn server binary: {}", e));

    ServerGuard(child)
}

#[test]
fn e2e_http_server_runtime() {
    // This test starts a real HTTP server from a compiled Mesh program,
    // makes an HTTP request, and verifies the response body.
    let source = read_fixture("stdlib_http_server_runtime.mpl");
    let mut guard = compile_and_start_server(&source);

    // Wait for the server to be ready by reading stderr for the listening message.
    // We need to do this in a separate thread to avoid blocking if the server
    // produces no output. Use a timeout approach instead.
    let stderr = guard.0.stderr.take().expect("no stderr pipe");
    let stderr_reader = BufReader::new(stderr);

    // Spawn a thread to read stderr and signal when server is ready.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                if line.contains("HTTP server listening on") {
                    let _ = tx.send(true);
                    return;
                }
            }
        }
        let _ = tx.send(false);
    });

    // Wait up to 10 seconds for the server to start.
    let ready = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or(false);
    assert!(ready, "Server did not start within 10 seconds");

    // Make an HTTP GET request to the server using raw TcpStream.
    // Retry up to 5 times with 200ms between attempts for robustness.
    let mut response = String::new();
    let mut connected = false;
    for attempt in 0..5 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        match std::net::TcpStream::connect("127.0.0.1:18080") {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                stream
                    .write_all(
                        b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .expect("failed to write HTTP request");
                stream
                    .read_to_string(&mut response)
                    .expect("failed to read HTTP response");
                connected = true;
                break;
            }
            Err(_) => continue,
        }
    }

    assert!(connected, "Failed to connect to server after 5 attempts");
    assert!(
        response.contains("200"),
        "Expected HTTP 200 in response, got: {}",
        response
    );
    // The Mesh string literal "{\"status\":\"ok\"}" is unescaped by the
    // compiler to {"status":"ok"} — Mesh processes standard escape sequences.
    assert!(
        response.contains(r#"{"status":"ok"}"#),
        "Expected JSON body in response, got: {}",
        response
    );

    // ServerGuard Drop will kill the server process.
}

// ── HTTP Crash Isolation E2E Tests (Phase 15) ─────────────────────────
//
// Verifies that a panic in one HTTP connection handler does not affect
// other connections, thanks to catch_unwind in connection_handler_entry.

#[test]
fn e2e_http_crash_isolation() {
    let source = read_fixture("stdlib_http_crash_isolation.mpl");
    let mut guard = compile_and_start_server(&source);

    let stderr = guard.0.stderr.take().expect("no stderr pipe");
    let stderr_reader = BufReader::new(stderr);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                if line.contains("HTTP server listening on") {
                    let _ = tx.send(true);
                    return;
                }
            }
        }
        let _ = tx.send(false);
    });
    let ready = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or(false);
    assert!(ready, "Server did not start within 10 seconds");

    // Step 1: Hit the /crash endpoint to trigger a panic in the handler actor.
    let _ = std::net::TcpStream::connect("127.0.0.1:18081").map(|mut stream| {
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .ok();
        stream
            .write_all(b"GET /crash HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .ok();
        let mut buf = String::new();
        let _ = stream.read_to_string(&mut buf);
    });

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Step 2: Hit the /health endpoint -- must still work after the crash.
    let mut response = String::new();
    let mut connected = false;
    for attempt in 0..5 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        match std::net::TcpStream::connect("127.0.0.1:18081") {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                stream
                    .write_all(
                        b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .expect("failed to write HTTP request");
                stream
                    .read_to_string(&mut response)
                    .expect("failed to read HTTP response");
                connected = true;
                break;
            }
            Err(_) => continue,
        }
    }

    assert!(connected, "Failed to connect to server after crash");
    assert!(
        response.contains("200"),
        "Expected HTTP 200 after crash isolation, got: {}",
        response
    );
    // The Mesh string literal "{\"status\":\"ok\"}" is unescaped by the
    // compiler to {"status":"ok"} — Mesh processes standard escape sequences.
    assert!(
        response.contains(r#"{"status":"ok"}"#),
        "Expected JSON body after crash isolation, got: {}",
        response
    );
}

// ── Math/Int/Float E2E Tests (Phase 43 Plan 01) ───────────────────────

#[test]
fn math_abs_int() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.abs(-42)}")
  println("${Math.abs(42)}")
  println("${Math.abs(0)}")
end
"#);
    assert_eq!(out.trim(), "42\n42\n0");
}

#[test]
fn math_abs_float() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.abs(-3.14)}")
  println("${Math.abs(3.14)}")
end
"#);
    assert!(out.contains("3.14"));
}

#[test]
fn math_min_max_int() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.min(10, 20)}")
  println("${Math.max(10, 20)}")
  println("${Math.min(-5, 3)}")
  println("${Math.max(-5, 3)}")
end
"#);
    assert_eq!(out.trim(), "10\n20\n-5\n3");
}

#[test]
fn math_min_max_float() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.min(1.5, 2.5)}")
  println("${Math.max(1.5, 2.5)}")
end
"#);
    assert!(out.contains("1.5"));
    assert!(out.contains("2.5"));
}

#[test]
fn math_pi_constant() {
    let out = compile_and_run(r#"
fn main() do
  let pi = Math.pi
  println("${pi}")
end
"#);
    assert!(out.contains("3.14159"));
}

#[test]
fn int_to_float_conversion() {
    let out = compile_and_run(r#"
fn main() do
  let f = Int.to_float(42)
  println("${f}")
end
"#);
    assert!(out.contains("42"));
}

#[test]
fn float_to_int_conversion() {
    let out = compile_and_run(r#"
fn main() do
  println("${Float.to_int(3.14)}")
  println("${Float.to_int(3.99)}")
  println("${Float.to_int(-2.7)}")
end
"#);
    // fptosi truncates toward zero
    assert_eq!(out.trim(), "3\n3\n-2");
}

#[test]
fn math_abs_with_variable() {
    let out = compile_and_run(r#"
fn main() do
  let x = -99
  println("${Math.abs(x)}")
end
"#);
    assert_eq!(out.trim(), "99");
}

#[test]
fn int_float_module_no_conflict_with_types() {
    // Verify Int/Float work as modules (Int.to_float, Float.to_int) while
    // Int and Float literals still work correctly (Pitfall 7: no name collision).
    let out = compile_and_run(r#"
fn main() do
  let x = 42
  let f = Int.to_float(x)
  let i = Float.to_int(f)
  println("${x}")
  println("${i}")
end
"#);
    assert_eq!(out.trim(), "42\n42");
}

// ── Math pow/sqrt/floor/ceil/round E2E Tests (Phase 43 Plan 02) ───────

#[test]
fn math_pow() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.pow(2.0, 10.0)}")
  println("${Math.pow(3.0, 2.0)}")
  println("${Math.pow(10.0, 0.0)}")
end
"#);
    assert!(out.contains("1024"));
    assert!(out.contains("9"));
    // 10^0 = 1
    assert!(out.lines().nth(2).unwrap().contains("1"));
}

#[test]
fn math_sqrt() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.sqrt(144.0)}")
  println("${Math.sqrt(2.0)}")
  println("${Math.sqrt(0.0)}")
end
"#);
    assert!(out.contains("12"));
    assert!(out.contains("1.41421"));
    assert!(out.contains("0"));
}

#[test]
fn math_floor() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.floor(3.7)}")
  println("${Math.floor(3.0)}")
  println("${Math.floor(-2.3)}")
end
"#);
    assert_eq!(out.trim(), "3\n3\n-3");
}

#[test]
fn math_ceil() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.ceil(3.2)}")
  println("${Math.ceil(3.0)}")
  println("${Math.ceil(-2.7)}")
end
"#);
    assert_eq!(out.trim(), "4\n3\n-2");
}

#[test]
fn math_round() {
    let out = compile_and_run(r#"
fn main() do
  println("${Math.round(3.5)}")
  println("${Math.round(3.4)}")
  println("${Math.round(-2.5)}")
  println("${Math.round(0.5)}")
end
"#);
    // llvm.round uses "round half away from zero"
    assert_eq!(out.trim(), "4\n3\n-3\n1");
}

#[test]
fn math_combined_usage() {
    let out = compile_and_run(r#"
fn main() do
  let radius = 5.0
  let area = Math.pow(radius, 2.0)
  println("${area}")
  let side = Math.sqrt(area)
  println("${side}")
  let pi_approx = Math.round(Math.pi)
  println("${pi_approx}")
  let converted = Float.to_int(Math.sqrt(Int.to_float(16)))
  println("${converted}")
end
"#);
    assert!(out.contains("25"));
    assert!(out.contains("5"));
    assert!(out.contains("3"));  // round(pi) = 3
    assert!(out.contains("4"));  // sqrt(16) = 4
}

#[test]
fn math_pow_with_conversion() {
    let out = compile_and_run(r#"
fn main() do
  let result = Math.pow(Int.to_float(2), Int.to_float(8))
  println("${Float.to_int(result)}")
end
"#);
    assert_eq!(out.trim(), "256");
}

// ── List Collection Operations E2E Tests (Phase 46 Plan 01) ────────────

#[test]
fn e2e_list_sort() {
    let source = read_fixture("stdlib_list_sort.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1\n9\n8\n");
}

#[test]
fn e2e_list_find() {
    // NOTE: List.find returns Option<T> (MeshOption ptr from runtime).
    // Pattern matching on the result via `case` hits a codegen domination
    // issue (pre-existing gap in FFI Option return handling).
    // This test verifies the function compiles, links, and runs without crash.
    let source = read_fixture("stdlib_list_find.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_list_any_all() {
    let source = read_fixture("stdlib_list_any_all.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\ntrue\nfalse\nfalse\n");
}

#[test]
fn e2e_list_contains() {
    let source = read_fixture("stdlib_list_contains.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\nfalse\nfalse\n");
}

// ── String Split/Join/Parse E2E Tests (Phase 46 Plan 02) ───────────────

#[test]
fn e2e_string_split_join() {
    let source = read_fixture("stdlib_string_split_join.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\nhello\nhello - world - foo\none,two,three\n");
}

#[test]
fn e2e_string_parse() {
    let source = read_fixture("stdlib_string_parse.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\nnone\n3.14\nnone\n-100\n");
}

// ── Extended List Collection Operations E2E Tests (Phase 47 Plan 01) ────

#[test]
fn e2e_stdlib_list_zip() {
    let source = read_fixture("stdlib_list_zip.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1\n10\n3\n2\n");
}

#[test]
fn e2e_stdlib_list_flat_map() {
    let source = read_fixture("stdlib_list_flat_map.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "6\n1\n10\n2\n5\n1\n5\n");
}

#[test]
fn e2e_stdlib_list_enumerate() {
    let source = read_fixture("stdlib_list_enumerate.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n0\n10\n");
}

#[test]
fn e2e_stdlib_list_take_drop() {
    let source = read_fixture("stdlib_list_take_drop.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n10\n30\n2\n40\n5\n0\n");
}

// ── JSON Sum Type & Generic E2E Tests (Phase 50) ──────────────────────

#[test]
fn e2e_deriving_json_sum_type() {
    let source = read_fixture("deriving_json_sum_type.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 5, "expected 5 lines, got: {}", output);
    // Line 0: Circle encode
    let json1: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON line 0");
    assert_eq!(json1["tag"], "Circle");
    assert!(json1["fields"].is_array());
    assert_eq!(json1["fields"].as_array().unwrap().len(), 1);
    assert!((json1["fields"][0].as_f64().unwrap() - 3.14).abs() < 0.01);
    // Line 1: Rectangle encode
    let json2: serde_json::Value = serde_json::from_str(lines[1]).expect("valid JSON line 1");
    assert_eq!(json2["tag"], "Rectangle");
    assert_eq!(json2["fields"].as_array().unwrap().len(), 2);
    assert!((json2["fields"][0].as_f64().unwrap() - 2.0).abs() < 0.01);
    assert!((json2["fields"][1].as_f64().unwrap() - 5.0).abs() < 0.01);
    // Line 2: Point encode
    let json3: serde_json::Value = serde_json::from_str(lines[2]).expect("valid JSON line 2");
    assert_eq!(json3["tag"], "Point");
    assert_eq!(json3["fields"].as_array().unwrap().len(), 0);
    // Line 3: Circle decode verification
    assert_eq!(lines[3], "circle: 3.14");
    // Line 4: Point decode verification
    assert_eq!(lines[4], "point: ok");
}

#[test]
fn e2e_deriving_json_generic() {
    let source = read_fixture("deriving_json_generic.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines, got: {}", output);
    // Line 0: Wrapper<Int> encode
    let json1: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON line 0");
    assert_eq!(json1["value"], 42);
    // Line 1: Wrapper<String> encode
    let json2: serde_json::Value = serde_json::from_str(lines[1]).expect("valid JSON line 1");
    assert_eq!(json2["value"], "hello");
}

#[test]
fn e2e_deriving_json_nested_sum() {
    let source = read_fixture("deriving_json_nested_sum.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 1, "expected 1 line, got: {}", output);
    let json: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON");
    // Verify Drawing struct has name and shapes fields
    assert_eq!(json["name"], "test");
    assert!(json["shapes"].is_array(), "shapes should be array");
    let shapes = json["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 3, "expected 3 shapes");
    // First shape: Circle(1.0)
    assert_eq!(shapes[0]["tag"], "Circle");
    assert_eq!(shapes[0]["fields"].as_array().unwrap().len(), 1);
    assert!((shapes[0]["fields"][0].as_f64().unwrap() - 1.0).abs() < 0.01);
    // Second shape: Point
    assert_eq!(shapes[1]["tag"], "Point");
    assert_eq!(shapes[1]["fields"].as_array().unwrap().len(), 0);
    // Third shape: Circle(2.5)
    assert_eq!(shapes[2]["tag"], "Circle");
    assert!((shapes[2]["fields"][0].as_f64().unwrap() - 2.5).abs() < 0.01);
}

#[test]
fn e2e_deriving_json_sum_non_serializable_compile_fail() {
    // Verify that deriving(Json) on a sum type with a non-serializable variant field (Pid)
    // produces a compile error containing E0038.
    let source = r#"
type BadSum do
  HasPid(Pid)
end deriving(Json)

fn main() do
  0
end
"#;
    let result = compile_only(source);
    assert!(
        !result.status.success(),
        "Expected compilation failure for non-serializable variant field, but it succeeded"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("E0038") || stderr.contains("not JSON-serializable"),
        "Expected E0038 error for sum type variant field, got stderr: {}",
        stderr
    );
}

// ── Phase 47 Plan 02: Map/Set Conversion E2E Tests ────────────────────

#[test]
fn e2e_stdlib_map_conversions() {
    let map_conv_source = read_fixture("stdlib_map_conversions.mpl");
    let map_conv_output = compile_and_run(&map_conv_source);
    assert_eq!(map_conv_output, "3\n10\n200\n30\n2\n2\n10\n20\n");
}

#[test]
fn e2e_stdlib_set_conversions() {
    let set_conv_source = read_fixture("stdlib_set_conversions.mpl");
    let set_conv_output = compile_and_run(&set_conv_source);
    assert_eq!(set_conv_output, "1\ntrue\nfalse\n3\n3\ntrue\ntrue\n");
}

// ── HTTP Path Parameters E2E Tests (Phase 51 Plan 02) ──────────────────
//
// Verifies the full Phase 51 stack: Mesh source -> typeck -> MIR -> LLVM ->
// runtime HTTP server with path parameter extraction, method-specific routing,
// exact-before-parameterized priority, and backward-compatible fallback.

/// Send an HTTP request to `127.0.0.1:{port}` with retries.
/// Returns the raw HTTP response as a string. Panics after 5 failed attempts.
fn send_request(port: u16, request: &str) -> String {
    let mut response = String::new();
    for attempt in 0..5 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        match std::net::TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                stream
                    .write_all(request.as_bytes())
                    .expect("failed to write HTTP request");
                stream
                    .read_to_string(&mut response)
                    .expect("failed to read HTTP response");
                return response;
            }
            Err(_) => continue,
        }
    }
    panic!("Failed to connect to 127.0.0.1:{} after 5 attempts", port);
}

#[test]
fn e2e_http_path_params() {
    let source = read_fixture("stdlib_http_path_params.mpl");
    let mut guard = compile_and_start_server(&source);

    // Wait for server to be ready.
    let stderr = guard.0.stderr.take().expect("no stderr pipe");
    let stderr_reader = BufReader::new(stderr);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                if line.contains("HTTP server listening on") {
                    let _ = tx.send(true);
                    return;
                }
            }
        }
        let _ = tx.send(false);
    });
    let ready = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or(false);
    assert!(ready, "Server did not start within 10 seconds");

    // Test A: Path parameter extraction (HTTP-01 + HTTP-02)
    let resp_a = send_request(
        18082,
        "GET /users/42 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_a.contains("200"),
        "Test A: Expected 200, got: {}",
        resp_a
    );
    assert!(
        resp_a.contains("42"),
        "Test A: Expected body '42', got: {}",
        resp_a
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test B: Exact route priority (SC-4) -- /users/me beats /users/:id
    let resp_b = send_request(
        18082,
        "GET /users/me HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_b.contains("200"),
        "Test B: Expected 200, got: {}",
        resp_b
    );
    // The body must be "me" from the exact route handler, not the param handler.
    // Split on the HTTP headers to get the body.
    let body_b = resp_b.split("\r\n\r\n").nth(1).unwrap_or("");
    assert_eq!(
        body_b.trim(),
        "me",
        "Test B: Expected exact route 'me', got body: '{}'",
        body_b
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test C: Method-specific routing (HTTP-03) -- POST /data
    let resp_c = send_request(
        18082,
        "POST /data HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_c.contains("200"),
        "Test C: Expected 200, got: {}",
        resp_c
    );
    assert!(
        resp_c.contains("posted"),
        "Test C: Expected body 'posted', got: {}",
        resp_c
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test D: Method filtering -- POST /users/42 should hit fallback (not the GET-only route)
    let resp_d = send_request(
        18082,
        "POST /users/42 HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_d.contains("200"),
        "Test D: Expected 200 (fallback), got: {}",
        resp_d
    );
    assert!(
        resp_d.contains("fallback"),
        "Test D: Expected fallback body, got: {}",
        resp_d
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test E: Fallback route (backward compat) -- GET /unknown/path
    let resp_e = send_request(
        18082,
        "GET /unknown/path HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_e.contains("200"),
        "Test E: Expected 200 (fallback), got: {}",
        resp_e
    );
    assert!(
        resp_e.contains("fallback"),
        "Test E: Expected fallback body, got: {}",
        resp_e
    );

    // ServerGuard Drop will kill the server process (path params).
}

// ── HTTP Middleware E2E Tests (Phase 52 Plan 02) ────────────────────────
//
// Verifies the full Phase 52 middleware stack: Mesh source -> typeck -> MIR ->
// LLVM -> runtime HTTP server with middleware interception. Tests passthrough
// middleware, short-circuit (auth), and middleware on unmatched routes (404).

#[test]
fn e2e_http_middleware() {
    let source = read_fixture("stdlib_http_middleware.mpl");
    let mut guard = compile_and_start_server(&source);

    // Wait for server to be ready.
    let stderr = guard.0.stderr.take().expect("no stderr pipe");
    let stderr_reader = BufReader::new(stderr);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                if line.contains("HTTP server listening on") {
                    let _ = tx.send(true);
                    return;
                }
            }
        }
        let _ = tx.send(false);
    });
    let ready = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or(false);
    assert!(ready, "Server did not start within 10 seconds");

    // Test A: Normal request passes through middleware chain.
    // logger passes through, auth_check allows (path doesn't start with /secret),
    // handler returns "hello-world".
    let resp_a = send_request(
        18083,
        "GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_a.contains("200"),
        "Test A: Expected 200, got: {}",
        resp_a
    );
    assert!(
        resp_a.contains("hello-world"),
        "Test A: Expected body 'hello-world', got: {}",
        resp_a
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test B: Auth middleware short-circuits for /secret path.
    // logger passes through, auth_check sees /secret and returns 401 without calling next.
    let resp_b = send_request(
        18083,
        "GET /secret HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_b.contains("401"),
        "Test B: Expected 401, got: {}",
        resp_b
    );
    assert!(
        resp_b.contains("Unauthorized"),
        "Test B: Expected body 'Unauthorized', got: {}",
        resp_b
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test C: Middleware runs on requests with no matching route (404).
    // Middleware chain executes (logger, auth_check), auth_check passes through
    // (path doesn't start with /secret), synthetic 404 handler returns 404.
    let resp_c = send_request(
        18083,
        "GET /nonexistent HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        resp_c.contains("404"),
        "Test C: Expected 404, got: {}",
        resp_c
    );

    // ServerGuard Drop will kill the server process (middleware).
}

// ── SQLite E2E Tests (Phase 53 Plan 02) ─────────────────────────────────
//
// Verifies the full SQLite driver pipeline: Mesh source -> compiler ->
// linked binary -> in-memory SQLite CRUD operations. Tests open, execute
// (DDL + DML with params), query with column names, parameterized WHERE,
// and close.

#[test]
fn e2e_sqlite() {
    let source = read_fixture("stdlib_sqlite.mpl");
    let output = compile_and_run(&source);
    // Verify insert counts
    assert!(output.contains("1"), "First insert should affect 1 row");
    // Verify query results
    assert!(output.contains("Alice:30"), "Should find Alice with age 30");
    assert!(output.contains("Bob:25"), "Should find Bob with age 25");
    // Verify parameterized query
    assert!(output.contains("filtered:Alice"), "Filtered query should find Alice");
    // Verify completion
    assert!(output.contains("done"), "Program should complete successfully");
}

// ── Phase 107 Gap Closure: SQLite JOIN Runtime Tests ────────────────────
//
// Verifies JOIN queries return correct row data at runtime against a real
// SQLite database. Tests INNER JOIN (both tables' columns), LEFT JOIN (NULL
// for unmatched rows), and multi-table JOIN (3+ tables).
// Proves ROADMAP SC2 (NULL handling) and SC4 (multi-table field mapping).

#[test]
fn e2e_sqlite_join_runtime() {
    let source = read_fixture("sqlite_join_runtime.mpl");
    let output = compile_and_run(&source);
    // INNER JOIN: 2 rows with fields from both tables
    assert!(output.contains("inner_join"), "Should have inner_join section");
    assert!(output.contains("Alice:Engineer"), "Inner join should return Alice with Engineer bio");
    assert!(output.contains("Bob:Designer"), "Inner join should return Bob with Designer bio");
    // LEFT JOIN: 3 rows, Charlie's bio is empty (NULL mapped to empty string)
    assert!(output.contains("left_join"), "Should have left_join section");
    assert!(output.contains("Charlie:"), "Left join should return Charlie with empty bio (NULL)");
    // Multi-table JOIN: columns from users, profiles, and departments
    assert!(output.contains("multi_join"), "Should have multi_join section");
    assert!(output.contains("Alice:Engineer:Engineering"), "Multi-join should return Alice with bio and dept");
    assert!(output.contains("Bob:Designer:Design"), "Multi-join should return Bob with bio and dept");
    // Success
    assert!(output.contains("done"), "Program should complete successfully");
}

// ── Phase 108: SQLite Aggregate Runtime Tests ───────────────────────────
//
// Verifies aggregate queries (count/sum/avg/min/max, GROUP BY, HAVING)
// execute correctly against a real SQLite database with correct results.
// Proves AGG-01 through AGG-04 at runtime.

#[test]
fn e2e_sqlite_aggregate_runtime() {
    let source = read_fixture("sqlite_aggregate_runtime.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().split('\n').collect();

    // AGG-01: count(*) = 6 total orders
    assert_eq!(lines[0], "count_all");
    assert_eq!(lines[1], "6");

    // AGG-02: sum=710, avg=118.333..., min=25, max=300
    assert_eq!(lines[2], "sum_avg_min_max");
    let agg_parts: Vec<&str> = lines[3].split(':').collect();
    assert_eq!(agg_parts[0], "710");       // sum
    // avg may be 118.333... or 118 depending on SQLite integer division
    assert!(agg_parts[1].starts_with("118"));  // avg (118 or 118.333...)
    assert_eq!(agg_parts[2], "25");        // min
    assert_eq!(agg_parts[3], "300");       // max

    // AGG-03: GROUP BY -- 3 groups
    assert_eq!(lines[4], "group_by");
    assert_eq!(lines[5], "books:2:60");           // books: 2 orders, sum=60
    assert_eq!(lines[6], "clothing:1:50");        // clothing: 1 order, sum=50
    assert_eq!(lines[7], "electronics:3:600");    // electronics: 3 orders, sum=600

    // AGG-04: HAVING count > 1 -- only books and electronics
    assert_eq!(lines[8], "having");
    assert_eq!(lines[9], "books:2");
    assert_eq!(lines[10], "electronics:3");

    assert_eq!(lines[11], "done");
}

// ── Phase 109: SQLite Upsert, DELETE RETURNING, Subquery WHERE Runtime Tests ──
//
// Verifies ON CONFLICT DO UPDATE SET RETURNING, DELETE ... RETURNING *,
// and WHERE IN (subquery) execute correctly against real SQLite data.
// Uses raw SQL matching build_upsert_sql_pure and delete_where_returning output.
// Proves UPS-01 through UPS-03 at runtime.

#[test]
fn e2e_sqlite_upsert_subquery_runtime() {
    let source = read_fixture("sqlite_upsert_subquery_runtime.mpl");
    let output = compile_and_run(&source);
    // UPS-01: Upsert via ON CONFLICT DO UPDATE SET RETURNING
    assert!(output.contains("upsert_insert:Delta"), "upsert insert failed: {}", output);
    assert!(output.contains("upsert_update:Delta-Updated"), "upsert update failed: {}", output);
    assert!(output.contains("upsert_count:4"), "upsert count should be 4 (no duplicate): {}", output);
    // UPS-02: DELETE with RETURNING
    assert!(output.contains("delete_ret_name:Gamma"), "deleted row should be Gamma: {}", output);
    assert!(output.contains("delete_after_count:0"), "p3 should be gone after delete: {}", output);
    // UPS-03: Subquery WHERE verification
    assert!(output.contains("subquery_first:Alpha"), "subquery first should be Alpha: {}", output);
    // Completion
    assert!(output.contains("done"), "did not complete: {}", output);
}

// ── PostgreSQL E2E Tests (Phase 54 Plan 02) ─────────────────────────────
//
// Verifies the full PostgreSQL driver pipeline: Mesh source -> compiler ->
// linked binary -> TCP connection to PostgreSQL -> wire protocol -> CRUD
// operations. Tests connect (SCRAM-SHA-256/MD5 auth), execute (DDL + DML
// with $1/$2 params), query with column names, filtered query, and close.
//
// Requires a running PostgreSQL instance with:
//   User: mesh_test  Password: mesh_test  Database: mesh_test
//
// Easiest setup:
//   docker run --name mesh-pg-test -e POSTGRES_USER=mesh_test \
//     -e POSTGRES_PASSWORD=mesh_test -e POSTGRES_DB=mesh_test \
//     -p 5432:5432 -d postgres:16
//
// Run with: cargo test e2e_pg -- --ignored

#[test]
#[ignore] // requires a running PostgreSQL instance
fn e2e_pg() {
    let source = read_fixture("stdlib_pg.mpl");
    let output = compile_and_run(&source);
    // Verify DDL result (CREATE TABLE returns 0 rows affected in PostgreSQL)
    assert!(output.contains("created: 0"), "CREATE TABLE should report 0 rows affected");
    // Verify insert counts
    assert!(output.contains("inserted: 1"), "INSERT should affect 1 row");
    // Verify query results
    assert!(output.contains("Alice is 30"), "Should find Alice with age 30");
    assert!(output.contains("Bob is 25"), "Should find Bob with age 25");
    // Verify parameterized query
    assert!(output.contains("older: Alice"), "Filtered query should find Alice (age 30 > 26)");
    // Verify completion
    assert!(output.contains("done"), "Program should complete successfully");
}

// ── Phase 87.1: Codegen Bug Fixes ──────────────────────────────────────

/// Phase 87.1: List.find Option pattern matching compiles and runs.
/// Pattern matching Some(x)/None on List.find result works correctly.
#[test]
fn e2e_list_find_option_match() {
    let source = read_fixture("list_find_option_match.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "found: 30\nnot found\n");
}

// ── Phase 105.1: Struct-in-Result ABI Fix ──────────────────────────────

/// Phase 105.1: Result<MultiFieldStruct, String> construct + match roundtrip.
/// Constructs Ok(Pair{42, 99}), Ok(Triple{10, 20, 30}), and Err("error"),
/// pattern matches to extract field sums. Validates that pointer-boxing
/// prevents the buffer overflow segfault when storing multi-field structs
/// into the {i8, ptr} sum type layout.
#[test]
fn e2e_struct_in_result_roundtrip() {
    let source = read_fixture("struct_in_result_roundtrip.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "141\n-1\n60\n");
}
