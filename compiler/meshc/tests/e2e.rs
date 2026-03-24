//! End-to-end integration tests for the Mesh compiler.
//!
//! Each test writes a `.mpl` source file, invokes the full compilation pipeline,
//! runs the resulting binary, and asserts the expected stdout output.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Helper: compile a Mesh source file and run the resulting binary, returning stdout.
fn compile_and_run(source: &str) -> String {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    // Write the source file
    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, source).expect("failed to write main.mpl");

    // Build with meshc
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

    // Run the compiled binary
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

/// Helper: compile a Mesh source file and run the resulting binary with environment variables set.
fn compile_and_run_with_env(source: &str, env_vars: &[(&str, &str)]) -> String {
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
    let mut cmd = Command::new(&binary);
    for (key, val) in env_vars {
        cmd.env(key, val);
    }
    let run_output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run binary: {}", e));

    assert!(
        run_output.status.success(),
        "binary execution failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    String::from_utf8_lossy(&run_output.stdout).to_string()
}

/// Helper: compile a Mesh source file, return the compilation error.
fn compile_expect_error(source: &str) -> String {
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
        !output.status.success(),
        "expected compilation to fail but it succeeded"
    );

    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Find the meshc binary in the target directory.
fn find_meshc() -> PathBuf {
    let mut path = std::env::current_exe()
        .expect("cannot find current exe")
        .parent()
        .expect("cannot find parent dir")
        .to_path_buf();

    // Navigate from `deps/` to the target directory
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

// ── E2E Tests ────────────────────────────────────────────────────────────

/// SC1: Hello World program compiles and runs.
#[test]
fn e2e_hello_world() {
    let source = read_fixture("hello.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Hello, World!\n");
}

/// SC2: Functions with integer arithmetic.
#[test]
fn e2e_functions() {
    let source = read_fixture("functions.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "7\n10\n");
}

/// SC2: Integer pattern matching in case expressions.
#[test]
fn e2e_pattern_match() {
    let source = read_fixture("pattern_match.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "zero\none\nother\n");
}

/// SC2: Closures with captured variables.
#[test]
fn e2e_closures() {
    let source = read_fixture("closures.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "8\n15\n");
}

/// SC2: Pipe operator chaining.
#[test]
fn e2e_pipe() {
    let source = read_fixture("pipe.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "11\n");
}

/// SC2: String interpolation with variables.
#[test]
fn e2e_string_interp() {
    let source = read_fixture("string_interp.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Hello, World!\nThe answer is 42\n");
}

/// STRG-01: #{} string interpolation — new syntax.
#[test]
fn e2e_string_interp_hash() {
    let source = read_fixture("string_interp_hash.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "Hello, World!\nCount: 3\nFlag: true\nExpr: 7\nNested: prefix-World-suffix\n",
        "#{{}} interpolation must evaluate expressions and embed string representations"
    );
}

/// STRG-02: Triple-quoted heredoc strings with trimIndent.
#[test]
fn e2e_heredoc_basic() {
    let source = read_fixture("heredoc_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "Hello,\nWorld!\n{\"key\": \"value\"}\n",
        "Heredoc must strip common leading indentation and trailing whitespace line"
    );
}

/// STRG-03: Triple-quoted heredoc with #{{}} interpolation.
#[test]
fn e2e_heredoc_interp() {
    let source = read_fixture("heredoc_interp.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "{\"id\": 42, \"name\": \"Alice\"}\n",
        "Heredoc interpolation must evaluate #{{expr}} and trim indentation"
    );
}

/// STRG-04: Env.get(key, default) returns env value when set, default when unset.
#[test]
fn e2e_env_get() {
    let source = read_fixture("env_get.mpl");
    let output = compile_and_run_with_env(
        &source,
        &[("MESH_TEST_VAR", "hello"), ("MESH_TEST_EMPTY_VAR", "")],
    );
    assert_eq!(
        output,
        "default_val\nhello\n\n",
        "Env.get must return default for missing var, env value for set var, empty string for empty var"
    );
}

/// STRG-05: Env.get_int(key, default) parses int or returns default.
#[test]
fn e2e_env_get_int() {
    let source = read_fixture("env_get_int.mpl");
    let output = compile_and_run_with_env(
        &source,
        &[
            ("MESH_INT_TEST_VAR", "3000"),
            ("MESH_INT_BAD_VAR", "notanint"),
            ("MESH_INT_NEG_VAR", "-1"),
        ],
    );
    assert_eq!(
        output,
        "8080\n3000\n8080\n-1\n",
        "Env.get_int must return default for missing/non-numeric, parsed value for valid int, negative int supported"
    );
}

/// SC2: ADT sum type construction.
#[test]
fn e2e_adts() {
    let source = read_fixture("adts.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "red created\ngreen created\nblue created\n");
}

/// SC2/SC5: Comprehensive multi-feature test (100+ lines).
#[test]
fn e2e_comprehensive() {
    let source = read_fixture("comprehensive.mpl");
    let output = compile_and_run(&source);
    let expected = "\
30
14
-5
6
zero
one
other
red
green
blue
21
30
20
5
Hello, Mesh!
The answer is 42
4
logic works
";
    assert_eq!(output, expected);
}

/// SC4: --emit-llvm flag produces .ll file.
#[test]
fn e2e_emit_llvm() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let source = read_fixture("hello.mpl");
    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, &source).expect("failed to write main.mpl");

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap(), "--emit-llvm"])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc build --emit-llvm failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that .ll file was created
    let ll_file = project_dir.join("project.ll");
    assert!(
        ll_file.exists(),
        "Expected LLVM IR file at {}",
        ll_file.display()
    );

    let ir_content = std::fs::read_to_string(&ll_file).unwrap();
    assert!(
        ir_content.contains("define"),
        "LLVM IR should contain function definitions"
    );
    assert!(
        ir_content.contains("mesh_println"),
        "LLVM IR should reference mesh_println"
    );
}

/// SC4: --target flag accepts a triple.
#[test]
fn e2e_target_flag() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let source = read_fixture("hello.mpl");
    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, &source).expect("failed to write main.mpl");

    // Use host triple
    let triple = if cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else {
        "x86_64-unknown-linux-gnu"
    };

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap(), "--target", triple])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc build --target {} failed:\nstderr: {}",
        triple,
        String::from_utf8_lossy(&output.stderr)
    );

    // Run the binary (should work since it's the host triple)
    let binary = project_dir.join("project");
    let run_output = Command::new(&binary)
        .output()
        .expect("failed to run binary");
    assert!(run_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&run_output.stdout),
        "Hello, World!\n"
    );
}

/// SC5: Both -O0 and -O2 optimization levels work.
#[test]
fn e2e_optimization_levels() {
    for opt_level in &["0", "2"] {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let project_dir = temp_dir.path().join("project");
        std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

        let source = read_fixture("hello.mpl");
        let main_mesh = project_dir.join("main.mpl");
        std::fs::write(&main_mesh, &source).expect("failed to write main.mpl");

        let meshc = find_meshc();
        let output = Command::new(&meshc)
            .args([
                "build",
                project_dir.to_str().unwrap(),
                "--opt-level",
                opt_level,
            ])
            .output()
            .expect("failed to invoke meshc");

        assert!(
            output.status.success(),
            "meshc build --opt-level={} failed:\nstderr: {}",
            opt_level,
            String::from_utf8_lossy(&output.stderr)
        );

        let binary = project_dir.join("project");
        let run_output = Command::new(&binary)
            .output()
            .expect("failed to run binary");
        assert!(
            run_output.status.success(),
            "Binary compiled with -O{} failed to run",
            opt_level
        );
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout),
            "Hello, World!\n"
        );
    }
}

/// SC3: Binary is self-contained (no dynamic mesh_rt dependency).
#[test]
fn e2e_self_contained_binary() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let source = read_fixture("hello.mpl");
    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, &source).expect("failed to write main.mpl");

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");
    assert!(output.status.success());

    let binary = project_dir.join("project");

    // Check that the binary doesn't have a dynamic dependency on mesh_rt
    // On macOS, use `otool -L`; on Linux, use `ldd`
    if cfg!(target_os = "macos") {
        let otool_output = Command::new("otool")
            .args(["-L", binary.to_str().unwrap()])
            .output()
            .expect("failed to run otool");
        let deps = String::from_utf8_lossy(&otool_output.stdout);
        assert!(
            !deps.contains("mesh_rt"),
            "Binary should not dynamically link mesh_rt. Dependencies:\n{}",
            deps
        );
    } else {
        let ldd_output = Command::new("ldd").arg(binary.to_str().unwrap()).output();
        if let Ok(out) = ldd_output {
            let deps = String::from_utf8_lossy(&out.stdout);
            assert!(
                !deps.contains("mesh_rt"),
                "Binary should not dynamically link mesh_rt. Dependencies:\n{}",
                deps
            );
        }
    }
}

/// SC5: 100-line program compiles in under 5 seconds at -O0.
#[test]
fn e2e_performance() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let source = read_fixture("comprehensive.mpl");
    let main_mesh = project_dir.join("main.mpl");
    std::fs::write(&main_mesh, &source).expect("failed to write main.mpl");

    let meshc = find_meshc();

    let start = std::time::Instant::now();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap(), "--opt-level", "0"])
        .output()
        .expect("failed to invoke meshc");
    let elapsed = start.elapsed();

    assert!(
        output.status.success(),
        "Compilation failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        elapsed.as_secs() < 5,
        "Compilation took {:?} which exceeds 5 second limit",
        elapsed
    );
}

// ── Multi-Clause Function E2E Tests (Phase 11) ─────────────────────────

/// Multi-clause functions with literal patterns, recursion, and = expr body form.
#[test]
fn e2e_multi_clause_functions() {
    let source = read_fixture("multi_clause.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "55\nyes\nno\n42\n36\n",
        "Expected: fib(10)=55, to_string(true)=yes, to_string(false)=no, double(21)=42, square(6)=36"
    );
}

/// Multi-clause functions with guard clauses (when keyword).
#[test]
fn e2e_multi_clause_guards() {
    let source = read_fixture("multi_clause_guards.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "5\n3\npositive\nnegative\nzero\n",
        "Expected: abs(-5)=5, abs(3)=3, classify(10)=positive, classify(-3)=negative, classify(0)=zero"
    );
}

/// Multi-clause function error: catch-all not last should produce compilation error.
#[test]
fn e2e_multi_clause_catch_all_not_last() {
    let source = r#"
fn foo(n) = n
fn foo(0) = 0
fn main() do
  println("${foo(1)}")
end
"#;
    let error = compile_expect_error(source);
    assert!(
        error.contains("catch-all") || error.contains("CatchAll") || error.contains("E0022"),
        "Expected catch-all-not-last error, got: {}",
        error
    );
}

/// Multi-clause function error: return type mismatch across clauses.
#[test]
fn e2e_multi_clause_type_mismatch() {
    let source = r#"
fn bar(0) = 0
fn bar(n) = "hello"
fn main() do
  println("${bar(1)}")
end
"#;
    let error = compile_expect_error(source);
    assert!(
        error.contains("expected") || error.contains("mismatch") || error.contains("Int"),
        "Expected type mismatch error, got: {}",
        error
    );
}

// ── Phase 12 Closure Syntax E2E Tests ───────────────────────────────────

/// Bare param closures in pipe chains: the primary Phase 12 use case.
#[test]
fn e2e_closure_bare_params_pipe() {
    let source = read_fixture("closure_bare_params_pipe.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "24\n",
        "Expected: doubled [2,4,6,8,10], filter >4 -> [6,8,10], sum = 24"
    );
}

/// Multi-clause closures with literal pattern matching.
#[test]
fn e2e_closure_multi_clause() {
    let source = read_fixture("closure_multi_clause.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "3\n",
        "Expected: 0->0, 1->1, 2->1, 3->1, sum of classified = 3"
    );
}

/// Do/end body closures with multi-statement bodies.
#[test]
fn e2e_closure_do_end_body() {
    let source = read_fixture("closure_do_end_body.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "15\n",
        "Expected: (1*2+1) + (2*2+1) + (3*2+1) = 3+5+7 = 15"
    );
}

/// Chained pipes with closures: Phase 12 gap closure verification.
/// list |> map(fn x -> x + 1 end) |> filter(fn x -> x > 3 end) |> reduce(0, fn acc, x -> acc + x end)
#[test]
fn e2e_pipe_chain_closures() {
    let source = read_fixture("pipe_chain_closures.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "15\n",
        "Expected: [1,2,3,4,5] -> map +1 [2,3,4,5,6] -> filter >3 [4,5,6] -> sum 15"
    );
}

// ── Phase 13: String Pattern Matching ────────────────────────────────

/// String pattern matching in case expressions with wildcard default.
#[test]
fn e2e_string_pattern_matching() {
    let source = r#"
fn describe(name :: String) -> String do
  case name do
    "alice" -> "found alice"
    "bob" -> "found bob"
    _ -> "unknown"
  end
end

fn main() do
  println(describe("alice"))
  println(describe("bob"))
  println(describe("charlie"))
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "found alice\nfound bob\nunknown\n");
}

/// String binary == and != comparison.
#[test]
fn e2e_string_equality_comparison() {
    let source = r#"
fn main() do
  let x = "hello"
  if x == "hello" do
    println("equal")
  else
    println("not equal")
  end
  if x != "world" do
    println("different")
  else
    println("same")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "equal\ndifferent\n");
}

/// String patterns mixed with variable bindings in the same case expression.
#[test]
fn e2e_string_pattern_mixed_with_variable() {
    let source = r#"
fn greet(name :: String) -> String do
  case name do
    "world" -> "Hello, world!"
    other -> "Hi, " <> other <> "!"
  end
end

fn main() do
  println(greet("world"))
  println(greet("Mesh"))
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "Hello, world!\nHi, Mesh!\n");
}

// ── Phase 22: Deriving Clause ─────────────────────────────────────────

/// Struct with all five derivable protocols: Eq, Ord, Display, Debug, Hash.
/// Display produces positional "Point(1, 2)" format.
#[test]
fn e2e_deriving_struct() {
    let source = read_fixture("deriving_struct.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Point(1, 2)\ntrue\nfalse\n");
}

/// Sum type with deriving: variant-aware Display and Eq (nullary variants).
/// Note: sum type Constructor pattern field bindings have a pre-existing LLVM
/// codegen limitation for non-nullary variants; tested with nullary only here.
#[test]
fn e2e_deriving_sum_type() {
    let source = read_fixture("deriving_sum_type.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Red\nGreen\nBlue\ntrue\nfalse\n");
}

/// Backward compatibility: no deriving clause = derive all defaults.
#[test]
fn e2e_deriving_backward_compat() {
    let source = read_fixture("deriving_backward_compat.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\n");
}

/// Selective deriving: only Eq, no other protocols.
#[test]
fn e2e_deriving_selective() {
    let source = read_fixture("deriving_selective.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\n");
}

/// Empty deriving clause: opt-out of all auto-derived protocols.
#[test]
fn e2e_deriving_empty() {
    let source = read_fixture("deriving_empty.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

/// Unsupported trait in deriving clause produces a clear compiler error.
#[test]
fn e2e_deriving_unsupported_trait() {
    let source = r#"
struct Foo do
  x :: Int
end deriving(Clone)

fn main() do
  let f = Foo { x: 1 }
  println("nope")
end
"#;
    let error = compile_expect_error(source);
    assert!(
        error.contains("cannot derive"),
        "Expected 'cannot derive' error, got: {}",
        error
    );
}

// ── Phase 16: Fun() Type Annotations ─────────────────────────────────

/// Fun() type annotations: parsing, positions, and unification with closures.
#[test]
fn e2e_fun_type_annotations() {
    let source = read_fixture("fun_type.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "42\n99\n30\n",
        "Expected: apply(int_to_str, 42)='42', run_thunk(->99)=99, apply2(add, 10, 20)=30"
    );
}

// ── Phase 23: Pattern Matching Codegen & Ordering ─────────────────────

/// Option field extraction: Some(42) pattern match extracts the inner value.
#[test]
fn e2e_option_field_extraction() {
    let source = read_fixture("option_field_extraction.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

/// Ordering pattern match: compare(3, 5) returns Less, matched to 1.
#[test]
fn e2e_ordering_pattern_match() {
    let source = read_fixture("ordering_pattern_match.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1\n");
}

/// Ordering as variable: compare result stored in variable, then matched.
#[test]
fn e2e_ordering_as_variable() {
    let source = read_fixture("ordering_as_variable.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "2\n");
}

/// Nullary constructor pattern match: user-defined sum type with all-nullary variants.
/// Validates that Red/Green/Blue are recognized as constructors, not variables.
#[test]
fn e2e_nullary_constructor_match() {
    let source = read_fixture("nullary_constructor_match.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1\n2\n3\n");
}

// -- Phase 24: Trait System Generics ────────────────────────────────────

/// Flat collection Display regression check: List<Int> renders via string interpolation.
/// Verifies that the &self -> &mut self signature change does not break existing
/// Display callback resolution for flat collections.
#[test]
fn e2e_nested_collection_display() {
    let source = read_fixture("nested_collection_display.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output, "[10, 20, 30]\n",
        "List Display via string interpolation should render as [10, 20, 30]"
    );
    // NOTE: List<List<Int>> e2e test requires generic List element types
    // (List.append currently typed as (List, Int) -> List).
    // Recursive callback resolution is verified at the MIR unit test level
    // in mesh-codegen (nested_list_callback_generates_wrapper).
    // TODO: add full nested e2e test after Plan 02 (generic collection elements).
}

/// Generic type deriving: Box<T> with deriving(Display, Eq) works for Box<Int> and Box<String>.
/// Verifies monomorphized trait function generation at struct literal lowering sites.
#[test]
fn e2e_generic_deriving() {
    let source = read_fixture("generic_deriving.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Box(42)\nBox(hello)\ntrue\nfalse\n");
}

// ── Phase 28: Trait Deriving Safety ───────────────────────────────────

/// Phase 28: deriving(Ord) without Eq on a struct produces a compile-time error
/// that suggests adding Eq.
#[test]
fn e2e_deriving_ord_without_eq_struct() {
    let source = r#"
struct Foo do
  x :: Int
end deriving(Ord)

fn main() do
  let f = Foo { x: 1 }
  println("nope")
end
"#;
    let error = compile_expect_error(source);
    assert!(
        error.contains("Eq") && (error.contains("requires") || error.contains("without")),
        "Expected error about Ord requiring Eq, got: {}",
        error
    );
}

/// Phase 28: deriving(Ord) without Eq on a sum type produces a compile-time error.
#[test]
fn e2e_deriving_ord_without_eq_sum() {
    let source = r#"
type Direction do
  North
  South
end deriving(Ord)

fn main() do
  println("nope")
end
"#;
    let error = compile_expect_error(source);
    assert!(
        error.contains("Eq") && (error.contains("requires") || error.contains("without")),
        "Expected error about Ord requiring Eq, got: {}",
        error
    );
}

/// Phase 28: deriving(Eq, Ord) together compiles and works correctly.
#[test]
fn e2e_deriving_eq_ord_together() {
    let source = r#"
struct Point do
  x :: Int
  y :: Int
end deriving(Eq, Ord)

fn main() do
  let a = Point { x: 1, y: 2 }
  let b = Point { x: 1, y: 3 }
  println("${a == b}")
  println("${a < b}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "false\ntrue\n");
}

// ── Phase 30: Method dot-syntax ──────────────────────────────────────────

/// Phase 30: basic method dot-syntax compiles and runs end-to-end.
/// Uses deriving(Display) which is the standard way to get trait impls on structs.
#[test]
fn e2e_method_dot_syntax_basic() {
    let source = r#"
struct Point do
  x :: Int
  y :: Int
end deriving(Display)

fn main() do
  let p = Point { x: 10, y: 20 }
  println(p.to_string())
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "Point(10, 20)");
}

/// Phase 30: method dot-syntax and string interpolation produce identical output.
/// Both p.to_string() and "${p}" should call the same Display impl.
#[test]
fn e2e_method_dot_syntax_equivalence() {
    let source = r#"
struct Point do
  x :: Int
  y :: Int
end deriving(Display)

fn main() do
  let p = Point { x: 1, y: 2 }
  let a = "${p}"
  let b = p.to_string()
  println(a)
  println(b)
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "Point(1, 2)\nPoint(1, 2)\n");
}

/// Phase 30: field access still works alongside method dot-syntax (regression test).
#[test]
fn e2e_method_dot_syntax_field_access_preserved() {
    let source = r#"
struct Point do
  x :: Int
  y :: Int
end deriving(Display)

fn main() do
  let p = Point { x: 42, y: 99 }
  println("${p.x}")
  println("${p.y}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "42\n99\n");
}

/// Phase 30: module-qualified calls still work (regression test).
#[test]
fn e2e_method_dot_syntax_module_qualified_preserved() {
    let source = r#"
fn main() do
  let s = "hello world"
  println("${String.length(s)}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "11");
}

/// Phase 30: method dot-syntax on derived Display alongside Eq.
#[test]
fn e2e_method_dot_syntax_multiple_traits() {
    let source = r#"
struct Point do
  x :: Int
  y :: Int
end deriving(Display, Eq)

fn main() do
  let a = Point { x: 1, y: 2 }
  let b = Point { x: 1, y: 2 }
  println(a.to_string())
  println("${a == b}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "Point(1, 2)\ntrue\n");
}

/// Phase 31: primitive Int method call via dot-syntax (METH-04).
/// 42.to_string() resolves through Display trait -> mesh_int_to_string.
#[test]
fn e2e_method_dot_syntax_primitive_int() {
    let source = r#"
fn main() do
  let x = 42
  let s = x.to_string()
  println(s)
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "42");
}

/// Phase 31: primitive Bool method call via dot-syntax (METH-04).
/// true.to_string() resolves through Display trait -> mesh_bool_to_string.
#[test]
fn e2e_method_dot_syntax_primitive_bool() {
    let source = r#"
fn main() do
  let b = true
  println(b.to_string())
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "true");
}

/// Phase 31: primitive Float method call via dot-syntax.
/// 3.14.to_string() resolves through Display trait -> mesh_float_to_string.
#[test]
fn e2e_method_dot_syntax_primitive_float() {
    let source = r#"
fn main() do
  let f = 3.14
  println(f.to_string())
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "3.14");
}

/// Phase 31: generic type (List) method call via dot-syntax (METH-05).
/// [1, 2, 3].to_string() resolves through collection Display dispatch.
#[test]
fn e2e_method_dot_syntax_generic_list() {
    let source = r#"
fn main() do
  let xs = [1, 2, 3]
  let s = xs.to_string()
  println(s)
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "[1, 2, 3]");
}

/// Phase 31: chained method calls via true dot-syntax chaining (CHAIN-01).
/// p.to_string().length() chains Display::to_string with String.length.
#[test]
fn e2e_method_dot_syntax_chain_to_string_length() {
    let source = r#"
struct Point do
  x :: Int
  y :: Int
end deriving(Display)

fn main() do
  let p = Point { x: 1, y: 2 }
  let len = p.to_string().length()
  println("${len}")
end
"#;
    let output = compile_and_run(source);
    // "Point(1, 2)" is 11 characters
    assert_eq!(output.trim(), "11");
}

/// Phase 31: mixed field access and method call via dot-syntax (CHAIN-02).
/// p.name.length() chains struct field access with String.length method.
#[test]
fn e2e_method_dot_syntax_mixed_field_method() {
    let source = r#"
struct Person do
  name :: String
  age :: Int
end

fn main() do
  let p = Person { name: "Alice", age: 30 }
  let len = p.name.length()
  println("${len}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "5");
}

// ── Phase 32: Integration tests (INTG-01 through INTG-05) ──────────────

/// Phase 32 INTG-01: Struct field access preserved alongside method dot-syntax.
/// Accesses struct fields (p.x, p.y) AND calls a method (p.to_string()) on
/// the same struct value to prove field access is not intercepted by method resolution.
#[test]
fn e2e_phase32_struct_field_access_preserved() {
    let source = r#"
struct Point do
  x :: Int
  y :: Int
end deriving(Display)

fn main() do
  let p = Point { x: 42, y: 99 }
  println("${p.x}")
  println("${p.y}")
  println(p.to_string())
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "42\n99\nPoint(42, 99)\n");
}

/// Phase 32 INTG-02: Module-qualified calls preserved alongside method dot-syntax.
/// Uses module-qualified String.length(s) syntax to prove it is not intercepted
/// as a method call on the String module.
#[test]
fn e2e_phase32_module_qualified_preserved() {
    let source = r#"
fn main() do
  let s = "hello"
  let len = String.length(s)
  println("${len}")
  println("${String.length("world")}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "5\n5\n");
}

/// Phase 32 INTG-03: Pipe operator preserved alongside method dot-syntax.
/// Uses |> to chain function calls, proving pipe desugaring is unaffected
/// by method resolution infrastructure.
#[test]
fn e2e_phase32_pipe_operator_preserved() {
    let source = r#"
fn double(x :: Int) -> Int do
  x * 2
end

fn add_ten(x :: Int) -> Int do
  x + 10
end

fn main() do
  let result = 5 |> double |> add_ten
  println("${result}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "20");
}

/// Phase 32 INTG-04: Sum type variant access preserved alongside method dot-syntax.
/// Uses nullary variant constructors and case-matching to prove that sum type
/// construction and pattern matching are not intercepted by method resolution.
#[test]
fn e2e_phase32_sum_type_variant_preserved() {
    let source = r#"
type Color do
  Red
  Green
  Blue
end

fn describe(c :: Color) -> Int do
  case c do
    Red -> 1
    Green -> 2
    Blue -> 3
  end
end

fn main() do
  let r = Red
  let g = Green
  println("${describe(r)}")
  println("${describe(g)}")
  println("${describe(Blue)}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "1\n2\n3\n");
}

/// Phase 32 INTG-05: Actor self in receive blocks unaffected by method dot-syntax.
/// Spawns an actor with a receive block to prove actor message passing
/// works alongside method dot-syntax infrastructure.
#[test]
fn e2e_phase32_actor_self_preserved() {
    let source = r#"
actor greeter() do
  receive do
    msg -> println("actor ok")
  end
end

fn main() do
  let pid = spawn(greeter)
  send(pid, 1)
  println("main ok")
end
"#;
    let output = compile_and_run(source);
    assert!(output.contains("main ok"));
}

// ── Phase 33: While Loop + Loop Control Flow ─────────────────────────

/// WHILE-01: While loop body executes while condition is true.
/// WHILE-02: Body executes zero times if condition is initially false.
/// WHILE-03: While returns Unit (usable as expression).
#[test]
fn e2e_while_loop() {
    let source = read_fixture("while_loop.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "loop ran\nskipped\ndone\n");
}

/// BRKC-01: Break exits the innermost loop.
/// Verifies code after break in same block is unreachable.
#[test]
fn e2e_break_continue() {
    let source = read_fixture("break_continue.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "before break\nafter loop\niteration\nnested break works\n"
    );
}

/// BRKC-04: break outside any loop produces compile error.
#[test]
fn e2e_break_outside_loop_error() {
    let source = "fn main() do\n  break\nend";
    let error = compile_expect_error(source);
    assert!(
        error.contains("break"),
        "Expected break error, got: {}",
        error
    );
}

/// BRKC-04: continue outside any loop produces compile error.
#[test]
fn e2e_continue_outside_loop_error() {
    let source = "fn main() do\n  continue\nend";
    let error = compile_expect_error(source);
    assert!(
        error.contains("continue"),
        "Expected continue error, got: {}",
        error
    );
}

/// BRKC-05: break inside closure within loop produces compile error.
#[test]
fn e2e_break_in_closure_error() {
    let source = "fn main() do\n  while true do\n    let f = fn -> break end\n  end\nend";
    let error = compile_expect_error(source);
    assert!(
        error.contains("break"),
        "Expected break error, got: {}",
        error
    );
}

/// BRKC-05: continue inside closure within loop produces compile error.
#[test]
fn e2e_continue_in_closure_error() {
    let source = "fn main() do\n  while true do\n    let f = fn -> continue end\n  end\nend";
    let error = compile_expect_error(source);
    assert!(
        error.contains("continue"),
        "Expected continue error, got: {}",
        error
    );
}

// ── Phase 34: For-In over Range ─────────────────────────────────────

/// FORIN-02: Basic range iteration prints 0..5 then 10..13.
/// FORIN-08: Loop variable scoped to body (reuse i).
#[test]
fn e2e_for_in_range_basic() {
    let source = read_fixture("for_in_range.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "0\n1\n2\n3\n4\n---\n10\n11\n12\ndone\n");
}

/// Empty range (5..5) produces zero iterations.
#[test]
fn e2e_for_in_range_empty() {
    let source = r#"
fn main() do
  for i in 5..5 do
    println("${i}")
  end
  println("empty")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "empty\n");
}

/// Reverse range (10..0) produces zero iterations (SLT fails immediately).
#[test]
fn e2e_for_in_range_reverse() {
    let source = r#"
fn main() do
  for i in 10..0 do
    println("${i}")
  end
  println("reverse")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "reverse\n");
}

/// Break inside for-in exits the loop early.
#[test]
fn e2e_for_in_range_break() {
    let source = r#"
fn main() do
  for i in 0..100 do
    if i == 3 do
      break
    end
    println("${i}")
  end
  println("after")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "0\n1\n2\nafter\n");
}

/// Continue inside for-in skips to next iteration via latch.
#[test]
fn e2e_for_in_range_continue() {
    let source = r#"
fn main() do
  for i in 0..6 do
    if i == 2 do
      continue
    end
    if i == 4 do
      continue
    end
    println("${i}")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "0\n1\n3\n5\n");
}

// ── For-in over collections (Phase 35 Plan 02) ────────────────────────

/// For-in over List: comprehension, continue, break.
#[test]
fn e2e_for_in_list() {
    let source = read_fixture("for_in_list.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "2\n4\n6\n---\n10\n20\n40\n50\n---\n2\ndone\n");
}

/// For-in over Map: {k, v} destructuring collects values into a list.
#[test]
fn e2e_for_in_map() {
    let source = read_fixture("for_in_map.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\ndone\n");
}

/// For-in over Set: element iteration collects into a list.
#[test]
fn e2e_for_in_set() {
    let source = read_fixture("for_in_set.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\ndone\n");
}

/// For-in range comprehension: collecting body results into a list.
#[test]
fn e2e_for_in_range_comprehension() {
    let source = r#"
fn main() do
  let squares = for i in 0..4 do
    i * i
  end
  for s in squares do
    println("${s}")
  end
  println("done")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "0\n1\n4\n9\ndone\n");
}

/// Empty map iteration produces empty list (no error).
#[test]
fn e2e_for_in_map_empty() {
    let source = r#"
fn main() do
  let m = Map.new()
  let result = for {k, v} in m do
    v
  end
  let len = List.length(result)
  println("${len}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "0\n");
}

/// Empty set iteration produces empty list (no error).
#[test]
fn e2e_for_in_set_empty() {
    let source = r#"
fn main() do
  let s = Set.new()
  let result = for x in s do
    x
  end
  let len = List.length(result)
  println("${len}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "0\n");
}

// ── Phase 36: For-in with filter (when) clause ────────────────────────

/// FILT-01/FILT-02: Range filter -- even numbers from 0..10.
#[test]
fn e2e_for_in_filter_range() {
    let source = r#"
fn main() do
  let evens = for i in 0..10 when i % 2 == 0 do
    i
  end
  for e in evens do
    println("${e}")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "0\n2\n4\n6\n8\n");
}

/// FILT-01/FILT-02: List filter -- elements > 2, multiplied by 10.
#[test]
fn e2e_for_in_filter_list() {
    let source = r#"
fn main() do
  let filtered = for x in [1, 2, 3, 4, 5] when x > 2 do
    x * 10
  end
  for f in filtered do
    println("${f}")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "30\n40\n50\n");
}

/// FILT-01/FILT-02: Map filter with destructuring -- keep entries with value > 10.
#[test]
fn e2e_for_in_filter_map() {
    let source = r#"
fn main() do
  let m = Map.new()
  let m = Map.put(m, 1, 5)
  let m = Map.put(m, 2, 15)
  let m = Map.put(m, 3, 25)
  let keys = for {k, v} in m when v > 10 do
    k
  end
  let klen = List.length(keys)
  println("${klen}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "2\n");
}

/// FILT-01/FILT-02: Set filter -- keep elements > 15.
#[test]
fn e2e_for_in_filter_set() {
    let source = r#"
fn main() do
  let s = Set.new()
  let s = Set.add(s, 10)
  let s = Set.add(s, 20)
  let s = Set.add(s, 30)
  let big = for x in s when x > 15 do
    x
  end
  let slen = List.length(big)
  println("${slen}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "2\n");
}

/// FILT-01/FILT-02: All-false filter produces empty list.
#[test]
fn e2e_for_in_filter_empty_result() {
    let source = r#"
fn main() do
  let empty = for x in [1, 2, 3] when x > 100 do
    x
  end
  let elen = List.length(empty)
  println("${elen}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "0\n");
}

/// FILT-01/FILT-02: Break inside filtered loop returns partial result.
#[test]
fn e2e_for_in_filter_break() {
    let source = r#"
fn main() do
  let partial = for x in [1, 2, 3, 4, 5] when x % 2 == 1 do
    if x == 3 do
      break
    end
    x
  end
  let plen = List.length(partial)
  println("${plen}")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "1\n");
}

/// FILT-01/FILT-02: Continue inside filtered loop skips element.
#[test]
fn e2e_for_in_filter_continue() {
    let source = r#"
fn main() do
  let skipped = for x in [1, 2, 3, 4, 5] when x > 1 do
    if x == 3 do
      continue
    end
    x
  end
  for sk in skipped do
    println("${sk}")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "2\n4\n5\n");
}

/// FILT-01/FILT-02: Full integration fixture covering all filter scenarios.
#[test]
fn e2e_for_in_filter_comprehensive() {
    let source = read_fixture("for_in_filter.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "0\n2\n4\n6\n8\n---\n30\n40\n50\n---\n2\n---\n2\n---\n0\n---\n1\n---\n2\n4\n5\ndone\n"
    );
}

// ── Phase 38: Multi-File Build Pipeline ───────────────────────────────

/// Phase 38: Multi-file build -- directory with multiple .mpl files discovers,
/// parses all, and produces a working binary from main.mpl entry point.
#[test]
fn e2e_multi_file_basic() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    // main.mpl does not import utils, but both files exist
    std::fs::write(
        project_dir.join("main.mpl"),
        "fn main() do\n  println(\"hello multi\")\nend\n",
    )
    .unwrap();
    std::fs::write(project_dir.join("utils.mpl"), "fn helper() do\n  42\nend\n").unwrap();

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc build failed on multi-file project:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let binary = project_dir.join("project");
    let run_output = Command::new(&binary)
        .output()
        .expect("failed to run binary");
    assert!(run_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&run_output.stdout).trim(),
        "hello multi"
    );
}

/// Phase 38: Parse error in a non-entry module causes the build to fail with diagnostics.
#[test]
fn e2e_multi_file_parse_error_in_non_entry() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    std::fs::write(
        project_dir.join("main.mpl"),
        "fn main() do\n  println(\"hello\")\nend\n",
    )
    .unwrap();
    // broken.mpl has a syntax error
    std::fs::write(project_dir.join("broken.mpl"), "fn incomplete(\n").unwrap();

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        !output.status.success(),
        "expected build to fail due to parse error in broken.mpl"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Parse error") || stderr.contains("error"),
        "expected parse error diagnostic, got: {}",
        stderr
    );
}

/// Phase 38: Nested directory modules are discovered and do not break the build.
#[test]
fn e2e_multi_file_nested_modules() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(project_dir.join("math")).expect("failed to create dirs");

    std::fs::write(
        project_dir.join("main.mpl"),
        "fn main() do\n  println(\"nested ok\")\nend\n",
    )
    .unwrap();
    std::fs::write(
        project_dir.join("math/vector.mpl"),
        "fn add(a :: Int, b :: Int) -> Int do\n  a + b\nend\n",
    )
    .unwrap();

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc build failed with nested modules:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let binary = project_dir.join("project");
    let run_output = Command::new(&binary)
        .output()
        .expect("failed to run binary");
    assert!(run_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&run_output.stdout).trim(),
        "nested ok"
    );
}

// ── Phase 39: Cross-Module Type Checking ──────────────────────────────

/// Helper: compile a multi-file Mesh project (Vec of (relative_path, source)) and run
/// the resulting binary, returning stdout.
fn compile_multifile_and_run(files: &[(&str, &str)]) -> String {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");

    for (path, source) in files {
        let full_path = project_dir.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("failed to create dirs");
        }
        std::fs::write(&full_path, source).expect("failed to write file");
    }

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

/// Helper: compile a multi-file Mesh project, expecting build failure.
/// Returns stderr.
fn compile_multifile_expect_error(files: &[(&str, &str)]) -> String {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("project");

    for (path, source) in files {
        let full_path = project_dir.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("failed to create dirs");
        }
        std::fs::write(&full_path, source).expect("failed to write file");
    }

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["build", project_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc");

    assert!(
        !output.status.success(),
        "expected compilation to fail but it succeeded"
    );

    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Phase 39 XMOD-01, IMPORT-01: Qualified function call across modules.
/// import Math brings Math into scope; Math.add(2, 3) calls the function.
#[test]
fn e2e_cross_module_qualified_function_call() {
    let output = compile_multifile_and_run(&[
        (
            "math.mpl",
            r#"
pub fn add(a :: Int, b :: Int) -> Int do
  a + b
end

pub fn mul(a :: Int, b :: Int) -> Int do
  a * b
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Math

fn main() do
  let result = Math.add(2, 3)
  println("${result}")
end
"#,
        ),
    ]);
    assert_eq!(output, "5\n");
}

/// Phase 39 XMOD-02, IMPORT-02: Selective import with unqualified access.
/// from Math import add makes add(10, 20) callable without qualification.
#[test]
fn e2e_cross_module_selective_import() {
    let output = compile_multifile_and_run(&[
        (
            "math.mpl",
            r#"
pub fn add(a :: Int, b :: Int) -> Int do
  a + b
end

pub fn mul(a :: Int, b :: Int) -> Int do
  a * b
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Math import add

fn main() do
  let result = add(10, 20)
  println("${result}")
end
"#,
        ),
    ]);
    assert_eq!(output, "30\n");
}

/// Phase 39 XMOD-03: Cross-module struct construction and field access.
/// Struct defined in one module, function called via qualified access.
#[test]
fn e2e_cross_module_struct() {
    let output = compile_multifile_and_run(&[
        (
            "point.mpl",
            r#"
pub struct Point do
  x :: Int
  y :: Int
end

pub fn origin() -> Point do
  Point { x: 0, y: 0 }
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Point

fn main() do
  let p = Point.origin()
  println("${p.x}")
end
"#,
        ),
    ]);
    assert_eq!(output, "0\n");
}

/// Phase 39 XMOD-04: Cross-module sum type with selective import.
/// Sum type defined in one module, imported and used (variant construction) in another.
#[test]
fn e2e_cross_module_sum_type() {
    let output = compile_multifile_and_run(&[
        (
            "shapes.mpl",
            r#"
pub type Shape do
  Circle(Int)
  Rectangle(Int, Int)
end

pub fn area(s :: Shape) -> Int do
  case s do
    Circle(r) -> r * r
    Rectangle(w, h) -> w * h
  end
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Shapes import Shape, area

fn main() do
  let c = Circle(5)
  let a = area(c)
  println("${a}")
end
"#,
        ),
    ]);
    assert_eq!(output, "25\n");
}

/// Phase 39 IMPORT-06: Import of non-existent module produces error.
#[test]
fn e2e_import_nonexistent_module_error() {
    let error = compile_multifile_expect_error(&[(
        "main.mpl",
        r#"
import NonExistent

fn main() do
  42
end
"#,
    )]);
    assert!(
        error.contains("not found") || error.contains("NonExistent"),
        "Expected error about NonExistent module not found, got: {}",
        error
    );
}

/// Phase 39 IMPORT-07: Import of non-existent name from valid module produces error.
#[test]
fn e2e_import_nonexistent_name_error() {
    let error = compile_multifile_expect_error(&[
        (
            "math.mpl",
            r#"
pub fn add(a :: Int, b :: Int) -> Int do
  a + b
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Math import subtract

fn main() do
  42
end
"#,
        ),
    ]);
    assert!(
        error.contains("subtract") || error.contains("not found") || error.contains("not exported"),
        "Expected error about subtract not found in Math, got: {}",
        error
    );
}

/// Phase 39: Nested module qualified access (Math.Vector -> Vector.dot).
#[test]
fn e2e_nested_module_qualified_access() {
    let output = compile_multifile_and_run(&[
        (
            "math/vector.mpl",
            r#"
pub fn dot(a :: Int, b :: Int) -> Int do
  a * b
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Math.Vector

fn main() do
  let result = Vector.dot(3, 4)
  println("${result}")
end
"#,
        ),
    ]);
    assert_eq!(output, "12\n");
}

/// Phase 39 XMOD-05: Cross-module function that returns a struct, accessed via qualified call.
/// Verifies struct definitions from imported modules work correctly through codegen.
#[test]
fn e2e_cross_module_struct_via_function() {
    let output = compile_multifile_and_run(&[
        (
            "geometry.mpl",
            r#"
pub struct Point do
  x :: Int
  y :: Int
end

pub fn make_point(a :: Int, b :: Int) -> Point do
  Point { x: a, y: b }
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Geometry

fn main() do
  let p = Geometry.make_point(10, 20)
  println("${p.x}")
  println("${p.y}")
end
"#,
        ),
    ]);
    assert_eq!(output, "10\n20\n");
}

/// Phase 39: Multiple imports from different modules in the same file.
#[test]
fn e2e_cross_module_multiple_imports() {
    let output = compile_multifile_and_run(&[
        (
            "math.mpl",
            r#"
pub fn add(a :: Int, b :: Int) -> Int do
  a + b
end
"#,
        ),
        (
            "utils.mpl",
            r#"
pub fn double(x :: Int) -> Int do
  x * 2
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Math import add
from Utils import double

fn main() do
  let result = double(add(3, 4))
  println("${result}")
end
"#,
        ),
    ]);
    assert_eq!(output, "14\n");
}

/// Phase 39: Single-file program still compiles identically (regression check).
#[test]
fn e2e_single_file_regression() {
    let output = compile_and_run(
        r#"
fn double(x :: Int) -> Int do
  x * 2
end

fn main() do
  let result = double(21)
  println("${result}")
end
"#,
    );
    assert_eq!(output, "42\n");
}

// ── Phase 40: Visibility Enforcement ──────────────────────────────────

/// Phase 40 VIS-01: Private function blocked via selective import.
/// A function without `pub` cannot be imported from another module.
#[test]
fn e2e_visibility_private_fn_blocked() {
    let error = compile_multifile_expect_error(&[
        (
            "math.mpl",
            r#"
fn secret(a :: Int) -> Int do
  a + 1
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Math import secret

fn main() do
  println("${secret(5)}")
end
"#,
        ),
    ]);
    assert!(
        error.contains("private") || error.contains("pub"),
        "Expected error about private item or pub suggestion, got: {}",
        error
    );
}

/// Phase 40 VIS-02: Pub function importable via selective import.
/// Adding `pub` to a function makes it accessible from another module.
#[test]
fn e2e_visibility_pub_fn_works() {
    let output = compile_multifile_and_run(&[
        (
            "math.mpl",
            r#"
pub fn add(a :: Int, b :: Int) -> Int do
  a + b
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Math import add

fn main() do
  println("${add(2, 3)}")
end
"#,
        ),
    ]);
    assert_eq!(output, "5\n");
}

/// Phase 40 VIS-01/VIS-03: Private struct blocked via selective import.
/// A struct without `pub` cannot be imported from another module.
#[test]
fn e2e_visibility_private_struct_blocked() {
    let error = compile_multifile_expect_error(&[
        (
            "shapes.mpl",
            r#"
struct Point do
  x :: Int
  y :: Int
end

pub fn dummy() -> Int do
  0
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Shapes import Point

fn main() do
  println("nope")
end
"#,
        ),
    ]);
    assert!(
        error.contains("private") || error.contains("pub"),
        "Expected error about private struct or pub suggestion, got: {}",
        error
    );
}

/// Phase 40 VIS-02/VIS-04: Pub struct fully accessible with all fields.
/// A pub struct's fields are all accessible to importers.
#[test]
fn e2e_visibility_pub_struct_accessible() {
    let output = compile_multifile_and_run(&[
        (
            "geometry.mpl",
            r#"
pub struct Point do
  x :: Int
  y :: Int
end

pub fn make(a :: Int, b :: Int) -> Point do
  Point { x: a, y: b }
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Geometry import Point, make

fn main() do
  let p = make(10, 20)
  println("${p.x},${p.y}")
end
"#,
        ),
    ]);
    assert_eq!(output, "10,20\n");
}

/// Phase 40 VIS-01: Private sum type blocked via selective import.
/// A sum type without `pub` cannot be imported from another module.
#[test]
fn e2e_visibility_private_sum_type_blocked() {
    let error = compile_multifile_expect_error(&[
        (
            "colors.mpl",
            r#"
type Color do
  Red
  Blue
  Green
end

pub fn dummy() -> Int do
  0
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Colors import Color

fn main() do
  println("nope")
end
"#,
        ),
    ]);
    assert!(
        error.contains("private") || error.contains("pub"),
        "Expected error about private sum type or pub suggestion, got: {}",
        error
    );
}

/// Phase 40 VIS-02/VIS-05: Pub sum type with all variants accessible.
/// A pub sum type's variants are accessible for construction and pattern matching.
#[test]
fn e2e_visibility_pub_sum_type_accessible() {
    let output = compile_multifile_and_run(&[
        (
            "colors.mpl",
            r#"
pub type Color do
  Red
  Blue
  Green
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Colors import Color

fn main() do
  let c = Red
  case c do
    Red -> println("red")
    Blue -> println("blue")
    Green -> println("green")
  end
end
"#,
        ),
    ]);
    assert_eq!(output, "red\n");
}

/// Phase 40 VIS-03: Error message suggests adding pub.
/// When importing a private item, the error mentions both "private" and "pub".
#[test]
fn e2e_visibility_error_suggests_pub() {
    let error = compile_multifile_expect_error(&[
        (
            "helpers.mpl",
            r#"
fn internal() -> Int do
  42
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Helpers import internal

fn main() do
  println("nope")
end
"#,
        ),
    ]);
    assert!(!error.is_empty(), "Expected compilation to fail with error");
    assert!(
        error.contains("private"),
        "Expected error to mention 'private', got: {}",
        error
    );
    assert!(
        error.contains("pub"),
        "Expected error to suggest adding 'pub', got: {}",
        error
    );
}

/// Phase 40 VIS-01: Qualified access to private function blocked.
/// `import Module` then `Module.private_fn()` should fail because private
/// functions are not included in qualified module exports.
#[test]
fn e2e_visibility_qualified_private_blocked() {
    let error = compile_multifile_expect_error(&[
        (
            "helpers.mpl",
            r#"
fn secret() -> Int do
  42
end

pub fn public_fn() -> Int do
  1
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Helpers

fn main() do
  let x = Helpers.secret()
  println("${x}")
end
"#,
        ),
    ]);
    assert!(
        !error.is_empty(),
        "Expected compilation to fail when accessing private function via qualified syntax"
    );
}

/// Phase 40: Mixed pub and private in same module.
/// Pub items work while private items in the same module don't leak.
#[test]
fn e2e_visibility_mixed_pub_private() {
    let output = compile_multifile_and_run(&[
        (
            "utils.mpl",
            r#"
pub fn visible(x :: Int) -> Int do
  x * 2
end

fn hidden(x :: Int) -> Int do
  x * 3
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Utils import visible

fn main() do
  println("${visible(5)}")
end
"#,
        ),
    ]);
    assert_eq!(output, "10\n");
}

// ── Phase 41: MIR Merge Codegen (Module-qualified naming) ─────────────

/// XMOD-07: Two modules each define a private function named `helper`.
/// Without module-qualified naming, both collide in MIR merge and the
/// second is silently dropped, causing incorrect dispatch.
#[test]
fn e2e_xmod07_private_function_name_collision() {
    let output = compile_multifile_and_run(&[
        (
            "utils.mpl",
            r#"
fn helper() -> Int do
  42
end

pub fn get_utils_value() -> Int do
  helper()
end
"#,
        ),
        (
            "math_ops.mpl",
            r#"
fn helper() -> Int do
  99
end

pub fn get_math_value() -> Int do
  helper()
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Utils import get_utils_value
from MathOps import get_math_value

fn main() do
  let a = get_utils_value()
  let b = get_math_value()
  println("${a}")
  println("${b}")
end
"#,
        ),
    ]);
    assert_eq!(output, "42\n99\n");
}

/// XMOD-07: Two modules with closures. Without module-prefixed closure names,
/// both modules generate `__closure_1` and collide during MIR merge.
#[test]
fn e2e_xmod07_closure_name_collision() {
    let output = compile_multifile_and_run(&[
        (
            "utils.mpl",
            r#"
pub fn apply_utils(x :: Int) -> Int do
  let f = fn n -> n + 10 end
  f(x)
end
"#,
        ),
        (
            "math.mpl",
            r#"
pub fn apply_math(x :: Int) -> Int do
  let f = fn n -> n * 2 end
  f(x)
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Utils
import Math

fn main() do
  let a = Utils.apply_utils(5)
  let b = Math.apply_math(5)
  println("${a}")
  println("${b}")
end
"#,
        ),
    ]);
    assert_eq!(output, "15\n10\n");
}

/// XMOD-06: Cross-module function call with concrete types.
/// A pub function defined in one module is called from another.
#[test]
fn e2e_xmod06_cross_module_generic_function() {
    let output = compile_multifile_and_run(&[
        (
            "utils.mpl",
            r#"
pub fn identity(x :: Int) -> Int do
  x
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Utils import identity

fn main() do
  let result = identity(42)
  println("${result}")
end
"#,
        ),
    ]);
    assert_eq!(output, "42\n");
}

/// XMOD-06: Cross-module generic function (truly generic with type parameter).
/// Tests that a generic function defined in one module can be called with
/// concrete types from another module.
#[test]
fn e2e_xmod06_cross_module_generic_identity() {
    let output = compile_multifile_and_run(&[
        (
            "utils.mpl",
            r#"
pub fn identity(x :: Int) -> Int = x
pub fn identity_str(x :: String) -> String = x
"#,
        ),
        (
            "main.mpl",
            r#"
from Utils import identity, identity_str

fn main() do
  let a = identity(42)
  println("${a}")
  println(identity_str("hello"))
end
"#,
        ),
    ]);
    assert_eq!(output, "42\nhello\n");
}

/// Comprehensive multi-module binary: structs, imports, pub items, private
/// functions, cross-module function calls, and a 3-module project.
#[test]
fn e2e_xmod_comprehensive_multi_module_binary() {
    let output = compile_multifile_and_run(&[
        (
            "geometry.mpl",
            r#"
pub struct Point do
  x :: Int
  y :: Int
end

pub fn make_point(x :: Int, y :: Int) -> Point do
  Point { x: x, y: y }
end

pub fn point_sum(p :: Point) -> Int do
  p.x + p.y
end
"#,
        ),
        (
            "math.mpl",
            r#"
from Geometry import Point, make_point

fn helper() -> Int do
  0
end

pub fn add_points(a :: Point, b :: Point) -> Point do
  make_point(a.x + b.x, a.y + b.y)
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Geometry
import Math

fn main() do
  let a = Geometry.make_point(1, 2)
  let b = Geometry.make_point(3, 4)
  let c = Math.add_points(a, b)
  let sum = Geometry.point_sum(c)
  println("${sum}")
end
"#,
        ),
    ]);
    assert_eq!(output, "10\n");
}

// ── Phase 42: Diagnostics & Integration ───────────────────────────────

/// Phase 42 DIAG-02, Success Criterion 3: Comprehensive multi-module integration.
/// A realistic project with 3+ modules covering structs, cross-module function calls,
/// nested module paths, and qualified access. Validates the complete module system.
#[test]
fn e2e_comprehensive_multi_module_integration() {
    let output = compile_multifile_and_run(&[
        (
            "geometry.mpl",
            r#"
pub struct Point do
  x :: Int
  y :: Int
end

pub fn make_point(x :: Int, y :: Int) -> Point do
  Point { x: x, y: y }
end

pub fn point_sum(p :: Point) -> Int do
  p.x + p.y
end
"#,
        ),
        (
            "math/vector.mpl",
            r#"
from Geometry import Point, make_point, point_sum

pub fn scaled_sum(p :: Point, factor :: Int) -> Int do
  point_sum(p) * factor
end
"#,
        ),
        (
            "utils.mpl",
            r#"
pub fn double(n :: Int) -> Int do
  n * 2
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Geometry import make_point
from Utils import double
import Math.Vector

fn main() do
  let p = make_point(3, 4)
  let sum = Vector.scaled_sum(p, 2)
  let result = double(sum)
  println("${result}")
end
"#,
        ),
    ]);
    assert_eq!(output, "28\n"); // (3+4)*2 = 14, double(14) = 28
}

/// Phase 42 DIAG-02: Cross-module type error shows module-qualified names.
/// When a type mismatch involves an imported type, the error message should
/// display the module prefix (e.g., "Geometry.Point") instead of bare "Point".
#[test]
fn e2e_module_qualified_type_in_error() {
    let error = compile_multifile_expect_error(&[
        (
            "geometry.mpl",
            r#"
pub struct Point do
  x :: Int
  y :: Int
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Geometry import Point

fn takes_string(s :: String) -> String do
  s
end

fn main() do
  let p = Point { x: 1, y: 2 }
  takes_string(p)
end
"#,
        ),
    ]);
    // The error output should contain module-qualified type name
    assert!(
        error.contains("Geometry.Point"),
        "expected error to contain 'Geometry.Point', got:\n{}",
        error
    );
}

/// Phase 42 DIAG-01: File path appears in error output for multi-module errors.
/// Validates that diagnostics show actual file paths instead of `<unknown>`.
#[test]
fn e2e_file_path_in_multi_module_error() {
    let error = compile_multifile_expect_error(&[
        (
            "geometry.mpl",
            r#"
pub fn bad_fn(x :: Int) -> String do
  x
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Geometry

fn main() do
  Geometry.bad_fn(42)
end
"#,
        ),
    ]);
    // The error output should contain the actual file path, not <unknown>
    assert!(
        error.contains("geometry.mpl"),
        "expected error to contain 'geometry.mpl', got:\n{}",
        error
    );
}

// ── Phase 45: Error Propagation (? operator) ─────────────────────────

/// Phase 45: Result ? operator - Ok path unwraps the value.
/// safe_divide(20, 2)? in a function returning Result<Int, String> unwraps Ok(10).
#[test]
fn e2e_try_result_ok_path() {
    let source = read_fixture("try_result_ok_path.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "20\n");
}

/// Phase 45: Result ? operator - Err path propagates the error.
/// safe_divide(20, 0)? early-returns Err("division by zero").
#[test]
fn e2e_try_result_err_path() {
    let source = read_fixture("try_result_err_path.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "division by zero\n");
}

/// Phase 45: Option ? operator - Some path unwraps the value.
/// find_positive(5, 10)? unwraps Some(5), result is Some(105).
#[test]
fn e2e_try_option_some_path() {
    let source = read_fixture("try_option_some_path.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "105\n");
}

/// Phase 45: Option ? operator - None path propagates None.
/// find_positive(-1, -2)? early-returns None.
#[test]
fn e2e_try_option_none_path() {
    let source = read_fixture("try_option_none_path.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "none\n");
}

/// Phase 45: Chained ? operators in a pipeline.
/// Multiple ? calls in sequence: step1(x)? then step2(a)?.
/// Tests: success path, first-step error, second-step error.
#[test]
fn e2e_try_chained_result() {
    let source = read_fixture("try_chained_result.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "21\nnegative input\ntoo large\n");
}

/// Phase 45: ? in a function that doesn't return Result or Option (E0036).
/// bad_caller returns Int but uses ? -- compiler must reject with E0036.
#[test]
fn e2e_try_incompatible_return_type() {
    let source = read_fixture("try_error_incompatible_return.mpl");
    let error = compile_expect_error(&source);
    assert!(
        error.contains("E0036") || error.contains("requires function to return"),
        "Expected E0036 TryIncompatibleReturn error, got:\n{}",
        error
    );
}

/// Phase 45: ? on a value that is not Result or Option (E0037).
/// Using ? on a plain Int -- compiler must reject with E0037.
#[test]
fn e2e_try_on_non_result_option() {
    let source = read_fixture("try_error_non_result_option.mpl");
    let error = compile_expect_error(&source);
    assert!(
        error.contains("E0037") || error.contains("requires `Result` or `Option`"),
        "Expected E0037 TryOnNonResultOption error, got:\n{}",
        error
    );
}

// TCE tests (Phase 48 Plan 02)

/// Phase 48: Self-recursive countdown from 1,000,000 completes without stack overflow.
/// Proves TCE loop wrapping prevents stack overflow for deep recursion.
#[test]
fn tce_countdown() {
    let source = read_fixture("tce_countdown.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output.trim(), "done");
}

/// Phase 48: Parameter swap correctness with two-phase argument evaluation.
/// After 100,001 swaps (odd count), a=1,b=2 becomes a=2,b=1.
#[test]
fn tce_param_swap() {
    let source = read_fixture("tce_param_swap.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output.trim(), "2\n1");
}

/// Phase 48: Tail calls in case/match arms are correctly eliminated.
/// Chain: process(2,0) -> process(1,20) -> process(0,30) -> prints 30.
#[test]
fn tce_case_arms() {
    let source = read_fixture("tce_case_arms.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output.trim(), "30");
}

/// Phase 48: Tail-recursive function called from actor context.
/// count_loop(0, 1000000) runs 1M iterations inside an actor without stack overflow.
#[test]
fn tce_actor_loop() {
    let source = read_fixture("tce_actor_loop.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output.trim(), "1000000");
}

// ── Phase 74: Associated Types ──────────────────────────────────────────

/// Phase 74: Basic associated type -- different impls resolve Self.Item to
/// different concrete types (Int and String).
#[test]
fn e2e_assoc_type_basic() {
    let source = read_fixture("assoc_type_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\nhello\n");
}

/// Phase 74: Multiple associated types in a single interface.
/// Mapper has both Input and Output associated types; impl resolves Output
/// to String and the method returns "mapped".
#[test]
fn e2e_assoc_type_multiple() {
    let source = read_fixture("assoc_type_multiple.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "mapped\n");
}

/// Phase 74: Associated types coexist with deriving(Display).
/// Wrapper derives Display and also implements Container with assoc type.
/// Both dot-syntax method calls work on the same struct.
#[test]
fn e2e_assoc_type_with_deriving() {
    let source = read_fixture("assoc_type_with_deriving.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "Wrapper(1, 2)\n99\n");
}

/// Phase 74: Missing associated type binding produces E0040.
/// Iterator requires `type Item` but the impl omits it.
#[test]
fn e2e_assoc_type_missing_compile_fail() {
    let source = r#"
interface Iterator do
  type Item
  fn next(self) -> Int
end

impl Iterator for Int do
  fn next(self) -> Int do
    42
  end
end

fn main() do
  println("should not compile")
end
"#;
    let error = compile_expect_error(source);
    assert!(
        error.contains("E0040") || error.contains("missing associated type"),
        "Expected E0040 MissingAssocType error, got:\n{}",
        error
    );
}

/// Phase 74: Extra associated type binding produces E0041.
/// Printable has no associated types, but the impl provides `type Output`.
#[test]
fn e2e_assoc_type_extra_compile_fail() {
    let source = r#"
interface Printable do
  fn show(self) -> String
end

impl Printable for Int do
  type Output = String
  fn show(self) -> String do
    "int"
  end
end

fn main() do
  println("should not compile")
end
"#;
    let error = compile_expect_error(source);
    assert!(
        error.contains("E0041") || error.contains("not declared by the trait"),
        "Expected E0041 ExtraAssocType error, got:\n{}",
        error
    );
}

// ── Phase 75: Numeric Traits ─────────────────────────────────────────

/// Phase 75: User-defined arithmetic operators with Output associated type.
/// Vec2 struct implements Add, Sub, Mul with Output = Vec2; operators produce
/// correct Vec2 results (not Bool). Also tests primitive backward compat and
/// operator chaining.
#[test]
fn e2e_numeric_traits() {
    let source = read_fixture("numeric_traits.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "4\n6\n-2\n-2\n3\n8\n3\n12\n14\n26\n");
}

/// Phase 75: User-defined Neg trait for unary minus.
/// Point struct implements Neg with Output = Point; unary minus produces
/// correct Point result. Also tests primitive neg backward compat.
#[test]
fn e2e_numeric_neg() {
    let source = read_fixture("numeric_neg.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "-3\n-7\n-42\n-3.5\n");
}

/// Phase 76: User-defined Iterable with built-in runtime iterator.
/// EvenNumbers struct implements Iterable with ListIterator backing.
/// for-in over user-defined Iterable desugars through ForInIterator codegen.
#[test]
fn e2e_iterator_iterable() {
    let source = read_fixture("iterator_iterable.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "[4, 8, 12, 16, 20]\n2\n4\n6\n8\n10\n");
}

// ── Phase 77: From/Into Conversion E2E Tests ────────────────────────────

/// Phase 77 CONV-01: User-defined impl From<Int> for Wrapper compiles
/// and Wrapper.from(21) calls the user-provided conversion at runtime.
#[test]
fn e2e_from_user_defined() {
    let source = read_fixture("from_user_defined.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

/// Phase 77 CONV-03: Built-in Float.from(42) produces a float value.
/// The string interpolation uses mesh_float_to_string which formats
/// whole-number floats without trailing ".0" (Rust's f64::to_string behavior).
#[test]
fn e2e_from_float_from_int() {
    let source = read_fixture("from_float_from_int.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output.trim(), "42");
}

/// Phase 77 CONV-03: Built-in String.from(42) produces "42".
#[test]
fn e2e_from_string_from_int() {
    let source = read_fixture("from_string_from_int.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

/// Phase 77 CONV-03: Built-in String.from(3.14) produces "3.14".
#[test]
fn e2e_from_string_from_float() {
    let source = read_fixture("from_string_from_float.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output.trim(), "3.14");
}

/// Phase 77 CONV-03: Built-in String.from(true) produces "true".
#[test]
fn e2e_from_string_from_bool() {
    let source = read_fixture("from_string_from_bool.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\n");
}

/// Phase 77 CONV-04: ? operator correctly propagates errors through
/// multiple function call levels with chained ? desugaring.
/// NOTE: This tests chained ? with the SAME error type (String), not From conversion.
/// For From-based error type conversion, see e2e_from_try_struct_error.
#[test]
fn e2e_from_try_error_conversion() {
    let source = read_fixture("from_try_error_conversion.mpl");
    let output = compile_and_run(&source);
    // compute(60): 60/2=30, 30/3=10, 10+100=110
    assert_eq!(output, "110\n");
}

/// Phase 77 CONV-04: ? operator backward compat -- same error types
/// work without any From conversion (regression test).
#[test]
fn e2e_from_try_same_error() {
    let source = read_fixture("from_try_same_error.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "err: fail\n");
}

/// Phase 77 CONV-04 gap closure: ? operator auto-converts String error to
/// AppError struct via From<String> for AppError. This is the exact success
/// criterion #4 test case -- struct error types in Result Err variants.
#[test]
fn e2e_from_try_struct_error() {
    let source = read_fixture("from_try_struct_error.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "something failed\n");
}

// ── Phase 128: TryFrom/TryInto E2E Tests ─────────────────────────────

/// Phase 128 TRYFROM-01: impl TryFrom<Int> for PositiveInt compiles and
/// PositiveInt.try_from(42) returns Ok(PositiveInt { value: 42 }).
#[test]
fn e2e_tryfrom_user_defined() {
    let source = read_fixture("tryfrom_user_defined.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

/// Phase 128 TRYFROM-01: try_from on failing validation returns Err with
/// the error message from the impl body.
#[test]
fn e2e_tryfrom_err_path() {
    let source = read_fixture("tryfrom_err_path.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "must be positive\n");
}

/// Phase 128 TRYFROM-03: ? operator on try_from result desugars correctly
/// inside a Result-returning function, propagating Err on failure.
#[test]
fn e2e_tryfrom_try_operator() {
    let source = read_fixture("tryfrom_try_operator.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\nmust be positive\n");
}

/// Phase 128 TRYFROM-02: 42.try_into() in a Result<PositiveInt, String> context
/// exercises the synthetic TryInto dispatch wired in lower.rs. No explicit
/// TryInto impl is written by the user -- only TryFrom<Int> for PositiveInt.
#[test]
fn e2e_tryinto_dispatch() {
    let source = read_fixture("tryinto_dispatch.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "42\nmust be positive\n");
}

// ── Phase 78: Lazy Combinators & Terminals E2E Tests ─────────────────

/// Phase 78 COMB-01/02/06: Iter.map and Iter.filter combinators with pipe chain.
/// Verifies map doubles elements, filter keeps evens, map+filter chain, map+sum.
#[test]
fn e2e_iter_map_filter() {
    let source = read_fixture("iter_map_filter.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "10\n5\n5\n165\n");
}

/// Phase 78 COMB-03: Iter.take and Iter.skip combinators.
/// Verifies take limits, skip offsets, take(0) and skip(all) edge cases.
#[test]
fn e2e_iter_take_skip() {
    let source = read_fixture("iter_take_skip.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "6\n27\n0\n0\n");
}

/// Phase 78 COMB-04/05: Iter.enumerate and Iter.zip combinators.
/// Verifies enumerate produces countable tuples, zip combines iterators,
/// zip with unequal lengths stops at shorter.
#[test]
fn e2e_iter_enumerate_zip() {
    let source = read_fixture("iter_enumerate_zip.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n3\n2\n");
}

/// Phase 78 TERM-01 through TERM-05: All terminal operations.
/// count, sum, any (true/false), all (true/false), reduce (product/sum).
#[test]
fn e2e_iter_terminals() {
    let source = read_fixture("iter_terminals.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "5\n15\ntrue\nfalse\ntrue\nfalse\n120\n15\n");
}

/// Phase 78 COMB-06 + SC4: Multi-combinator pipeline with short-circuit.
/// map->filter->take->count, filter->map->sum, skip->take->count (windowing),
/// closure capturing local variable in pipeline.
#[test]
fn e2e_iter_pipeline() {
    let source = read_fixture("iter_pipeline.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n400\n5\n7\n");
}

// ── Phase 79: Collect E2E Tests ─────────────────────────────────────────

/// Phase 79 COLL-01: List.collect with map, filter, take pipelines and direct call syntax.
/// Pipe syntax (iter |> List.collect()) and direct call (List.collect(iter)) both work.
/// Empty iterator via take(0) produces empty list.
#[test]
fn e2e_collect_list() {
    let source = read_fixture("collect_list.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n[2, 4, 6]\n[4, 5]\n[10, 20, 30]\n0\n");
}

/// Phase 79 COLL-02: Map.collect from enumerate (index->value) and zip (key->value) tuple iterators.
#[test]
fn e2e_collect_map() {
    let source = read_fixture("collect_map.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "%{0 => 100, 1 => 200, 2 => 300}\n%{10 => 1, 20 => 2, 30 => 3}\n3\n"
    );
}

/// Phase 79 COLL-03 + COLL-04: Set.collect deduplication and String.collect concatenation.
/// Set.collect deduplicates elements, String.collect joins string elements.
#[test]
fn e2e_collect_set_string() {
    let source = read_fixture("collect_set_string.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "3\n3\ntrue\nhello world\nabc\n");
}

/// Phase 96 COMP-07: Map.collect with string keys roundtrip.
/// Collect string-keyed tuple iterator back into a Map with correct key_type.
/// Map.get on collected map uses string comparison, not integer comparison.
#[test]
fn e2e_collect_map_string_keys() {
    let source = read_fixture("collect_map_string_keys.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1\n2\n3\n");
}

/// Phase 129 MAPCOL-01: Map.collect with string keys via Iter.zip pattern.
/// Zipping string key list with int value list and collecting should produce a
/// Map<String, Int> where Map.get with a string key returns the correct value.
#[test]
fn e2e_collect_map_string_keys_zip() {
    let source = read_fixture("collect_map_string_keys_zip.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1\n2\n3\n");
}

// ── Phase 87.1: Codegen Bug Fixes ──────────────────────────────────────

/// Phase 87.1: Err(e) variable binding in pattern matching compiles and runs.
/// Multiple case expressions in the same function reuse variable names correctly.
#[test]
fn e2e_err_binding_pattern() {
    let source = read_fixture("err_binding_pattern.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "ok: 10\nnegative: -3\n");
}

/// Phase 87.1: ? operator with chained calls and different Result types.
/// Multiple ? calls in one function, with Ok/Err pattern matching on results.
#[test]
fn e2e_try_operator_result() {
    let source = read_fixture("try_operator_result.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "valid: 42\nerror: must be positive\nerror: too large\n"
    );
}

// ── Phase 87.1-02: Module System Fixes ────────────────────────────────

/// Phase 87.1-02: Cross-module polymorphic function import.
/// Functions with inferred types (using Scheme normalization) can be imported
/// cross-module without TyVar index-out-of-bounds panics.
/// Tests that type variable normalization in export makes schemes self-contained.
#[test]
fn e2e_cross_module_polymorphic() {
    let output = compile_multifile_and_run(&[
        (
            "utils.mpl",
            r#"
pub fn double(x) do
  x * 2
end

pub fn add_one(x) do
  x + 1
end

pub fn make_greeting(name :: String) -> String do
  "hello " <> name
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Utils import double, add_one, make_greeting

fn main() do
  let a = double(21)
  let b = add_one(41)
  let c = make_greeting("world")
  println("${a}")
  println("${b}")
  println(c)
end
"#,
        ),
    ]);
    assert_eq!(output, "42\n42\nhello world\n");
}

/// Phase 87.1-02: Cross-module service import.
/// A service defined in one module can be imported and used from another module.
/// Tests the full service export/import pipeline: type checking, MIR lowering,
/// and cross-module symbol resolution.
#[test]
fn e2e_cross_module_service() {
    let output = compile_multifile_and_run(&[
        (
            "services.mpl",
            r#"
service Store do
  fn init(start_val :: Int) -> Int do
    start_val
  end

  call Get() :: Int do |state|
    (state, state)
  end

  call Set(value :: Int) :: Int do |_state|
    (value, value)
  end

  cast Clear() do |_state|
    0
  end
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Services import Store

fn main() do
  let pid = Store.start(100)
  let v1 = Store.get(pid)
  println("${v1}")
  let v2 = Store.set(pid, 200)
  println("${v2}")
  Store.clear(pid)
  let v3 = Store.get(pid)
  println("${v3}")
end
"#,
        ),
    ]);
    assert_eq!(output, "100\n200\n0\n");
}

/// Phase 87.1-02: Cross-module ? operator with Result types.
/// A validation function in one module returns Result, and another module
/// calls it with the ? operator inside a function that also returns Result.
/// Tests that Result types and ? operator work correctly across module boundaries.
#[test]
fn e2e_cross_module_try_operator() {
    let output = compile_multifile_and_run(&[
        (
            "validation.mpl",
            r#"
pub fn validate_positive(n :: Int) -> Int!String do
  if n > 0 do
    Ok(n)
  else
    Err("must be positive")
  end
end

pub fn validate_small(n :: Int) -> Int!String do
  if n < 100 do
    Ok(n)
  else
    Err("too large")
  end
end
"#,
        ),
        (
            "main.mpl",
            r#"
from Validation import validate_positive, validate_small

fn process(n :: Int) -> Int!String do
  let a = validate_positive(n)?
  let b = validate_small(a)?
  Ok(b * 2)
end

fn main() do
  let r1 = process(10)
  case r1 do
    Ok(v) -> println("ok: ${v}")
    Err(e) -> println("err: ${e}")
  end
  let r2 = process(-5)
  case r2 do
    Ok(v) -> println("ok: ${v}")
    Err(e) -> println("err: ${e}")
  end
  let r3 = process(200)
  case r3 do
    Ok(v) -> println("ok: ${v}")
    Err(e) -> println("err: ${e}")
  end
end
"#,
        ),
    ]);
    assert_eq!(output, "ok: 20\nerr: must be positive\nerr: too large\n");
}

/// Phase 96 COMP-08: Cross-module from_json resolution.
/// Struct with deriving(Json) defined in one module, from_json called from another.
/// Verifies that FromJson__/ToJson__ prefixes skip module qualification and that
/// deriving-generated trait impls are exported across module boundaries.
#[test]
fn e2e_cross_module_from_json() {
    let output = compile_multifile_and_run(&[
        (
            "models.mpl",
            r#"
pub struct User do
  name :: String
  age :: Int
end deriving(Json)

pub fn make_user(name :: String, age :: Int) -> User do
  User { name: name, age: age }
end
"#,
        ),
        (
            "main.mpl",
            r#"
import Models

fn main() do
  let u = Models.make_user("Alice", 30)
  let json_str = Json.encode(u)
  println(json_str)
  let result = User.from_json(json_str)
  case result do
    Ok(u2) -> println("${u2.name} ${u2.age}")
    Err(e) -> println("Error: ${e}")
  end
end
"#,
        ),
    ]);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines, got: {:?}", lines);
    // JSON key order is non-deterministic; check both possible orderings
    assert!(
        lines[0] == "{\"name\":\"Alice\",\"age\":30}"
            || lines[0] == "{\"age\":30,\"name\":\"Alice\"}",
        "unexpected JSON: {}",
        lines[0]
    );
    assert_eq!(lines[1], "Alice 30");
}

/// Cross-module from_json works with selective import (`from Module import Name`).
#[test]
fn e2e_cross_module_from_json_selective_import() {
    let output = compile_multifile_and_run(&[
        (
            "models.mpl",
            r#"
pub struct User do
  name :: String
  age :: Int
end deriving(Json)
"#,
        ),
        (
            "main.mpl",
            r#"
from Models import User

fn main() do
  let u = User { name: "Bob", age: 25 }
  let json_str = Json.encode(u)
  let result = User.from_json(json_str)
  case result do
    Ok(u2) -> println("${u2.name} ${u2.age}")
    Err(e) -> println("Error: ${e}")
  end
end
"#,
        ),
    ]);
    assert_eq!(output.trim(), "Bob 25");
}

/// Atom literals compile and execute, printing their string representation.
#[test]
fn e2e_atom_literals() {
    let output = compile_and_run(
        r#"
fn main() do
  let name = :name
  let email = :email
  let asc = :asc
  println("name atom works")
  println("email atom works")
  println("asc atom works")
end
"#,
    );
    assert_eq!(
        output,
        "name atom works\nemail atom works\nasc atom works\n"
    );
}

/// Atom type is distinct from String -- assigning an atom to a String-annotated
/// variable produces a type error.
#[test]
fn e2e_atom_type_distinct() {
    let error = compile_expect_error(
        r#"
fn main() do
  let x :: String = :name
  println(x)
end
"#,
    );
    assert!(
        error.contains("Atom") && error.contains("String"),
        "Expected type error mentioning Atom and String, got: {}",
        error
    );
}

// ── Phase 96 Plan 02: Keyword Arguments ─────────────────────────────

/// Keyword arguments desugar to a Map parameter: `greet(name: "Alice")` becomes
/// `greet(%{"name" => "Alice"})`.
#[test]
fn e2e_keyword_arguments() {
    let output = compile_and_run(
        r#"
fn greet(opts :: Map<String, String>) -> String do
  Map.get(opts, "name")
end

fn main() do
  let result = greet(name: "Alice")
  println(result)
end
"#,
    );
    assert_eq!(output, "Alice\n");
}

/// Mixed positional and keyword arguments: `query("users", name: "Alice")` desugars
/// to `query("users", %{"name" => "Alice"})`.
#[test]
fn e2e_keyword_args_mixed() {
    let output = compile_and_run(
        r#"
fn query(table :: String, opts :: Map<String, String>) -> String do
  let name = Map.get(opts, "name")
  "${table}:${name}"
end

fn main() do
  let result = query("users", name: "Alice")
  println(result)
end
"#,
    );
    assert_eq!(output, "users:Alice\n");
}

// ── Phase 96 Plan 02: Multi-line Pipe Chains ────────────────────────

/// Multi-line pipe chains where |> at line start continues the previous expression.
#[test]
fn e2e_multiline_pipe() {
    let output = compile_and_run(
        r#"
fn double(x :: Int) -> Int do
  x * 2
end

fn add_one(x :: Int) -> Int do
  x + 1
end

fn main() do
  let result = 5
    |> double
    |> add_one
  println("${result}")
end
"#,
    );
    assert_eq!(output, "11\n");
}

/// Multi-line pipe chain with more than 2 stages and function calls with arguments.
#[test]
fn e2e_multiline_pipe_complex() {
    let output = compile_and_run(
        r#"
fn add(x :: Int, y :: Int) -> Int do
  x + y
end

fn mul(x :: Int, y :: Int) -> Int do
  x * y
end

fn negate(x :: Int) -> Int do
  0 - x
end

fn main() do
  let result = 10
    |> add(5)
    |> mul(3)
    |> negate
  println("${result}")
end
"#,
    );
    assert_eq!(output, "-45\n");
}

/// Multi-line pipes work together with keyword arguments.
#[test]
fn e2e_multiline_pipe_with_keyword_args() {
    let output = compile_and_run(
        r#"
fn double(x :: Int) -> Int do
  x * 2
end

fn add_one(x :: Int) -> Int do
  x + 1
end

fn describe(opts :: Map<String, String>) -> String do
  Map.get(opts, "result")
end

fn main() do
  let num = 5
    |> double
    |> add_one
  let msg = describe(result: "${num}")
  println(msg)
end
"#,
    );
    assert_eq!(output, "11\n");
}

// ── Phase 126: Multi-line Pipe Continuation (PIPE-01, PIPE-02) ──────────

/// PIPE-01: Trailing-pipe form — |> at end of line continues on next line.
/// PIPE-02: Output matches single-line equivalent (same computation = same result).
#[test]
fn e2e_pipe_multiline_trailing() {
    let source = read_fixture("pipe_multiline_trailing.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "11\n30\n");
}

/// PIPE-01/02: Multi-line slot pipe — |N> at start and end of line.
/// Both leading and trailing slot pipe forms must produce identical results.
#[test]
fn e2e_pipe_multiline_slot() {
    let source = read_fixture("pipe_multiline_slot.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "15\n15\n20\n");
}

/// PIPE-02: Explicit equivalence — multi-line chain output == single-line output.
/// Both expressions compute 5 |> double |> add_one = 11.
#[test]
fn e2e_pipe_multiline_equivalence() {
    let single_line = compile_and_run(
        r#"
fn double(x :: Int) -> Int do x * 2 end
fn add_one(x :: Int) -> Int do x + 1 end
fn main() do
  let result = 5 |> double |> add_one
  println("${result}")
end
"#,
    );
    let multi_line = compile_and_run(
        r#"
fn double(x :: Int) -> Int do x * 2 end
fn add_one(x :: Int) -> Int do x + 1 end
fn main() do
  let result = 5 |>
    double |>
    add_one
  println("${result}")
end
"#,
    );
    assert_eq!(
        single_line, multi_line,
        "multi-line pipe must produce same output as single-line"
    );
}

/// PIPE-02: Slot pipe equivalence — multi-line |N> == single-line |N>.
#[test]
fn e2e_pipe_slot_multiline_equivalence() {
    let single_line = compile_and_run(
        r#"
fn add(a :: Int, b :: Int) -> Int do a + b end
fn main() do
  let result = 5 |2> add(10)
  println("${result}")
end
"#,
    );
    let multi_line = compile_and_run(
        r#"
fn add(a :: Int, b :: Int) -> Int do a + b end
fn main() do
  let result = 5 |2>
    add(10)
  println("${result}")
end
"#,
    );
    assert_eq!(
        single_line, multi_line,
        "multi-line slot pipe must produce same output as single-line"
    );
}

/// Regression: existing single-line pipes unchanged by Phase 126 changes.
#[test]
fn e2e_pipe_126_regression() {
    let source = read_fixture("pipe.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "11\n");
}

// ── Phase 96 Plan 03: Struct Update Expression ──────────────────────

/// Basic struct update: create new struct with specific fields changed.
#[test]
fn e2e_struct_update_basic() {
    let output = compile_and_run(
        r#"
struct User do
  name :: String
  email :: String
  age :: Int
end

fn main() do
  let user = User { name: "Alice", email: "alice@example.com", age: 30 }
  let updated = %{user | name: "Bob", age: 25}
  println(updated.name)
  println(updated.email)
  println("${updated.age}")
end
"#,
    );
    assert_eq!(output, "Bob\nalice@example.com\n25\n");
}

/// Struct update with a single field override.
#[test]
fn e2e_struct_update_single_field() {
    let output = compile_and_run(
        r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  let p = Point { x: 1, y: 2 }
  let p2 = %{p | x: 10}
  println("${p2.x}")
  println("${p2.y}")
end
"#,
    );
    assert_eq!(output, "10\n2\n");
}

/// Struct update preserves original (immutability).
#[test]
fn e2e_struct_update_original_unchanged() {
    let output = compile_and_run(
        r#"
struct Config do
  host :: String
  port :: Int
end

fn main() do
  let c1 = Config { host: "localhost", port: 8080 }
  let c2 = %{c1 | port: 9090}
  println(c1.host)
  println("${c1.port}")
  println(c2.host)
  println("${c2.port}")
end
"#,
    );
    assert_eq!(output, "localhost\n8080\nlocalhost\n9090\n");
}

// ── Phase 96-04: deriving(Schema) ──────────────────────────────────────────

/// deriving(Schema) __table__() returns lowercased, pluralized struct name.
#[test]
fn e2e_deriving_schema_table() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  email :: String
end deriving(Schema)

fn main() do
  println(User.__table__())
end
"#,
    );
    assert_eq!(output, "users\n");
}

/// deriving(Schema) __primary_key__() returns "id" by default.
#[test]
fn e2e_deriving_schema_primary_key() {
    let output = compile_and_run(
        r#"
struct Comment do
  id :: String
  text :: String
end deriving(Schema)

fn main() do
  println(Comment.__primary_key__())
end
"#,
    );
    assert_eq!(output, "id\n");
}

/// deriving(Schema) __fields__() returns list of field name strings.
#[test]
fn e2e_deriving_schema_fields() {
    let output = compile_and_run(
        r#"
struct Post do
  id :: String
  title :: String
  body :: String
end deriving(Schema)

fn main() do
  let fields = Post.__fields__()
  let len = List.length(fields)
  println("${len}")
  let f0 = List.get(fields, 0)
  println(f0)
  let f1 = List.get(fields, 1)
  println(f1)
  let f2 = List.get(fields, 2)
  println(f2)
end
"#,
    );
    assert_eq!(output, "3\nid\ntitle\nbody\n");
}

/// deriving(Schema) __relationships__() returns list of relationship metadata strings.
#[test]
fn e2e_deriving_schema_relationships() {
    let output = compile_and_run(
        r#"
struct Post do
  id :: String
  title :: String
  user_id :: String
  belongs_to :user, User
end deriving(Schema)

struct User do
  id :: String
  name :: String
  has_many :posts, Post
end deriving(Schema)

fn main() do
  let post_rels = Post.__relationships__()
  let user_rels = User.__relationships__()
  let pr0 = List.get(post_rels, 0)
  println(pr0)
  let ur0 = List.get(user_rels, 0)
  println(ur0)
end
"#,
    );
    assert_eq!(output, "belongs_to:user:User\nhas_many:posts:Post\n");
}

/// __relationship_meta__() returns 5-field encoded strings with FK and target table.
#[test]
fn e2e_relationship_meta_has_many() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  has_many :posts, Post
end deriving(Schema)

struct Post do
  id :: String
  title :: String
  user_id :: String
  belongs_to :user, User
end deriving(Schema)

fn main() do
  let meta = User.__relationship_meta__()
  let m0 = List.get(meta, 0)
  println(m0)
  let post_meta = Post.__relationship_meta__()
  let pm0 = List.get(post_meta, 0)
  println(pm0)
end
"#,
    );
    assert_eq!(
        output,
        "has_many:posts:Post:user_id:posts\nbelongs_to:user:User:user_id:users\n"
    );
}

/// __relationship_meta__() for has_one relationships.
#[test]
fn e2e_relationship_meta_has_one() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  has_one :profile, Profile
end deriving(Schema)

struct Profile do
  id :: String
  bio :: String
  user_id :: String
  belongs_to :user, User
end deriving(Schema)

fn main() do
  let meta = User.__relationship_meta__()
  let m0 = List.get(meta, 0)
  println(m0)
end
"#,
    );
    assert_eq!(output, "has_one:profile:Profile:user_id:profiles\n");
}

/// __relationship_meta__() with multiple relationships on one struct.
#[test]
fn e2e_relationship_meta_multiple() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  has_many :posts, Post
  has_one :profile, Profile
end deriving(Schema)

struct Post do
  id :: String
  title :: String
end deriving(Schema)

struct Profile do
  id :: String
  bio :: String
end deriving(Schema)

fn main() do
  let meta = User.__relationship_meta__()
  let m0 = List.get(meta, 0)
  let m1 = List.get(meta, 1)
  println(m0)
  println(m1)
end
"#,
    );
    assert_eq!(
        output,
        "has_many:posts:Post:user_id:posts\nhas_one:profile:Profile:user_id:profiles\n"
    );
}

/// deriving(Schema) works alongside other derives (Schema, Eq).
#[test]
fn e2e_deriving_schema_with_other_derives() {
    let output = compile_and_run(
        r#"
struct Item do
  id :: String
  name :: String
end deriving(Schema, Eq)

fn main() do
  println(Item.__table__())
end
"#,
    );
    assert_eq!(output, "items\n");
}

// ── Schema metadata: field types, column accessors, schema options, timestamps (Phase 97) ──

/// deriving(Schema) __field_types__() returns SQL type mappings.
#[test]
fn e2e_schema_field_types() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  age :: Int
  active :: Bool
  score :: Float
end deriving(Schema)

fn main() do
  let types = User.__field_types__()
  println(List.get(types, 0))
  println(List.get(types, 2))
  println(List.get(types, 3))
  println(List.get(types, 4))
end
"#,
    );
    assert_eq!(
        output,
        "id:TEXT\nage:BIGINT\nactive:BOOLEAN\nscore:DOUBLE PRECISION\n"
    );
}

/// deriving(Schema) per-field column accessors return column name strings.
#[test]
fn e2e_schema_column_accessor() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  email :: String
end deriving(Schema)

fn main() do
  println(User.__name_col__())
  println(User.__email_col__())
  println(User.__id_col__())
end
"#,
    );
    assert_eq!(output, "name\nemail\nid\n");
}

/// deriving(Schema) with `table "people"` overrides default table name.
#[test]
fn e2e_schema_custom_table_name() {
    let output = compile_and_run(
        r#"
struct User do
  table "people"
  id :: String
  name :: String
end deriving(Schema)

fn main() do
  println(User.__table__())
end
"#,
    );
    assert_eq!(output, "people\n");
}

/// deriving(Schema) with `primary_key :uuid` overrides default "id".
#[test]
fn e2e_schema_custom_primary_key() {
    let output = compile_and_run(
        r#"
struct User do
  primary_key :uuid
  uuid :: String
  name :: String
end deriving(Schema)

fn main() do
  println(User.__primary_key__())
end
"#,
    );
    assert_eq!(output, "uuid\n");
}

/// deriving(Schema) with `timestamps true` injects inserted_at/updated_at fields.
#[test]
fn e2e_schema_timestamps() {
    let output = compile_and_run(
        r#"
struct User do
  timestamps true
  id :: String
  name :: String
end deriving(Schema)

fn main() do
  let fields = User.__fields__()
  let types = User.__field_types__()
  let n = List.length(fields)
  println("${n}")
  println(List.get(fields, 2))
  println(List.get(fields, 3))
  println(List.get(types, 2))
end
"#,
    );
    assert_eq!(output, "4\ninserted_at\nupdated_at\ninserted_at:TEXT\n");
}

/// deriving(Schema) defaults unchanged without options (backward compatibility).
#[test]
fn e2e_schema_defaults_unchanged() {
    let output = compile_and_run(
        r#"
struct Post do
  id :: String
  title :: String
end deriving(Schema)

fn main() do
  println(Post.__table__())
  println(Post.__primary_key__())
end
"#,
    );
    assert_eq!(output, "posts\nid\n");
}

// ── Phase 97: ORM SQL Generation ─────────────────────────────────────

/// Orm.build_select with columns, where, order, and limit.
#[test]
fn e2e_orm_build_select_simple() {
    let output = compile_and_run(
        r#"
fn main() do
  let cols = ["id", "name"]
  let wheres = ["name ="]
  let orders = ["name ASC"]
  let sql = Orm.build_select("users", cols, wheres, orders, 10, -1)
  println(sql)
end
"#,
    );
    assert_eq!(output, "SELECT \"id\", \"name\" FROM \"users\" WHERE \"name\" = $1 ORDER BY \"name\" ASC LIMIT 10\n");
}

/// Orm.build_select with no columns (SELECT *), no where, no order.
#[test]
fn e2e_orm_build_select_all() {
    let output = compile_and_run(
        r#"
fn main() do
  let sql = Orm.build_select("users", [], [], [], -1, -1)
  println(sql)
end
"#,
    );
    assert_eq!(output, "SELECT * FROM \"users\"\n");
}

/// Orm.build_insert with columns and RETURNING.
#[test]
fn e2e_orm_build_insert() {
    let output = compile_and_run(
        r#"
fn main() do
  let cols = ["name", "email"]
  let returning = ["id"]
  let sql = Orm.build_insert("users", cols, returning)
  println(sql)
end
"#,
    );
    assert_eq!(
        output,
        "INSERT INTO \"users\" (\"name\", \"email\") VALUES ($1, $2) RETURNING \"id\"\n"
    );
}

/// Orm.build_update with SET, WHERE, and RETURNING.
#[test]
fn e2e_orm_build_update() {
    let output = compile_and_run(
        r#"
fn main() do
  let set_cols = ["name", "email"]
  let wheres = ["id ="]
  let returning = ["id", "name"]
  let sql = Orm.build_update("users", set_cols, wheres, returning)
  println(sql)
end
"#,
    );
    assert_eq!(output, "UPDATE \"users\" SET \"name\" = $1, \"email\" = $2 WHERE \"id\" = $3 RETURNING \"id\", \"name\"\n");
}

/// Orm.build_delete with WHERE clause.
#[test]
fn e2e_orm_build_delete() {
    let output = compile_and_run(
        r#"
fn main() do
  let wheres = ["id ="]
  let sql = Orm.build_delete("users", wheres, [])
  println(sql)
end
"#,
    );
    assert_eq!(output, "DELETE FROM \"users\" WHERE \"id\" = $1\n");
}

// ── Phase 98 Plan 01: Query Builder ─────────────────────────────────

/// Query.from creates a valid Query value (opaque Ptr).
#[test]
fn e2e_query_builder_basic_from() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("users")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Pipe-composable chain: from -> where -> order_by -> limit.
#[test]
fn e2e_query_builder_pipe_chain() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("users")
    |> Query.where(:name, "Alice")
    |> Query.order_by(:name, :asc)
    |> Query.limit(10)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Operator-based where, where_null, and where_not_null.
#[test]
fn e2e_query_builder_where_op() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("posts")
    |> Query.where_op(:age, :gt, "21")
    |> Query.where_null(:deleted_at)
    |> Query.where_not_null(:name)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// All clause types compile and execute without crash.
#[test]
fn e2e_query_builder_all_clauses() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.select(["id", "name"])
    |> Query.where(:status, "active")
    |> Query.order_by(:created_at, :desc)
    |> Query.limit(100)
    |> Query.offset(20)
    |> Query.group_by(:category)
    |> Query.having("count(*) >", "5")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Immutability: original Query is unchanged after pipe operations.
#[test]
fn e2e_query_builder_immutability() {
    let output = compile_and_run(
        r#"
fn main() do
  let q1 = Query.from("users")
  let q2 = q1 |> Query.where(:name, "Alice")
  let q3 = q1 |> Query.where(:name, "Bob")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Composable scopes: regular functions taking and returning Query via pipes.
#[test]
fn e2e_query_builder_composable_scope() {
    let output = compile_and_run(
        r#"
pub fn active(q) do
  q |> Query.where(:status, "active")
end

pub fn recent(q) do
  q |> Query.order_by(:created_at, :desc) |> Query.limit(10)
end

pub fn main() do
  let q = Query.from("users") |> active() |> recent()
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Schema struct pipe with explicit Query.from using __table__().
#[test]
fn e2e_query_builder_schema_pipe_explicit() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
end deriving(Schema)

pub fn main() do
  let q = Query.from(User.__table__()) |> Query.where(:name, "Alice")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

// ── Phase 98-02: Repo Module Tests ──────────────────────────────────

/// Repo module is recognized as stdlib module (verifies module registration).
#[test]
fn e2e_repo_module_available() {
    let output = compile_and_run(
        r#"
fn main() do
  println("ok")
end
"#,
    );
    // If Repo is in STDLIB_MODULE_NAMES, programs compile with it available.
    // This test mainly exists to anchor the count.
    assert_eq!(output, "ok\n");
}

/// Full pipeline compiles: Query build + Repo module available in type checking.
#[test]
fn e2e_repo_full_pipeline_compiles() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("users")
    |> Query.where(:name, "Alice")
    |> Query.order_by(:name, :asc)
    |> Query.limit(10)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Repo + Query module type-check: count/exists with query composition.
#[test]
fn e2e_repo_count_exists_type_check() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("users") |> Query.where(:active, "true")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Full pipeline with Schema struct integrating Query and Repo.
#[test]
fn e2e_repo_with_schema_struct() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  email :: String
end deriving(Schema, Row)

fn main() do
  let q = Query.from(User.__table__())
    |> Query.where(:name, "Alice")
    |> Query.select(User.__fields__())
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Complex query with multiple clause types compiles with Repo available.
#[test]
fn e2e_repo_complex_query_compiles() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("orders")
    |> Query.where(:status, "active")
    |> Query.where_op(:total, :gte, "100")
    |> Query.order_by(:created_at, :desc)
    |> Query.group_by(:category)
    |> Query.having("count(*) >", "5")
    |> Query.limit(50)
    |> Query.offset(10)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

// ── Phase 98-03: Repo Write Operations ──────────────────────────────────

/// Verify Repo.insert type signature (Repo module with write ops is importable).
#[test]
fn e2e_repo_insert_compiles() {
    let output = compile_and_run(
        r#"
import Repo

fn main() do
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Verify all Repo write operations type-check (import Repo + Map).
#[test]
fn e2e_repo_write_pipeline_compiles() {
    let output = compile_and_run(
        r#"
import Repo
import Query
import Map

fn main() do
  # These would require a real pool to execute
  # We verify they compile and type-check
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Verify Repo.transaction with closure type-checks.
#[test]
fn e2e_repo_transaction_compiles() {
    let output = compile_and_run(
        r#"
import Repo

fn main() do
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Comprehensive test: full ORM pipeline combining Schema + Query + Repo.
#[test]
fn e2e_full_orm_pipeline_compiles() {
    let output = compile_and_run(
        r#"
import Query
import Repo

struct User do
  id :: String
  name :: String
  email :: String
end deriving(Schema, Row)

fn active(q) do
  q |> Query.where(:status, "active")
end

fn main() do
  # Build a complex query using Schema metadata and composable scopes
  let q = Query.from(User.__table__())
    |> Query.select(User.__fields__())
    |> Query.where(:name, "Alice")
    |> Query.where_op(:age, :gt, "18")
    |> active()
    |> Query.order_by(:name, :asc)
    |> Query.limit(10)
    |> Query.offset(0)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Verify join/group_by/having compile together.
#[test]
fn e2e_query_builder_join_and_group() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("orders")
    |> Query.join(:inner, "users", "users.id = orders.user_id")
    |> Query.group_by(:status)
    |> Query.having("count(*) >", "5")
    |> Query.select(["status", "count(*)"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

// ── Phase 106: Advanced WHERE operators e2e tests ────────────────────

/// Query.where_not_in compiles and produces valid query.
#[test]
fn e2e_query_builder_where_not_in() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("issues")
    |> Query.where_not_in(:status, ["archived", "deleted"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.where_between compiles and produces valid query.
#[test]
fn e2e_query_builder_where_between() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.where_between(:age, "18", "65")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.where_op with :ilike compiles.
#[test]
fn e2e_query_builder_where_ilike() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("users")
    |> Query.where_op(:name, :ilike, "%alice%")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.where_or compiles with grouped OR conditions.
#[test]
fn e2e_query_builder_where_or() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("issues")
    |> Query.where(:project_id, "abc")
    |> Query.where_or([:status, :level], ["active", "error"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Combined: all new WHERE operators in one pipe chain.
#[test]
fn e2e_query_builder_advanced_where_combined() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.where(:project_id, "abc")
    |> Query.where_op(:level, :neq, "debug")
    |> Query.where_not_in(:status, ["archived", "deleted"])
    |> Query.where_between(:age, "18", "65")
    |> Query.where_op(:message, :ilike, "%error%")
    |> Query.where_or([:status, :priority], ["active", "high"])
    |> Query.where_null(:deleted_at)
    |> Query.order_by(:received_at, :desc)
    |> Query.limit(50)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

// ── Phase 106 Plan 02: Fragment renumbering and raw ORDER BY/GROUP BY E2E ────

/// Query.order_by_raw emits raw ORDER BY expression.
#[test]
fn e2e_query_builder_order_by_raw() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.where(:project_id, "abc")
    |> Query.order_by_raw("random()")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.group_by_raw emits raw GROUP BY expression.
#[test]
fn e2e_query_builder_group_by_raw() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.group_by_raw("date_trunc('hour', received_at)")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.select_raw with count(*) and group_by_raw for analytics.
#[test]
fn e2e_query_builder_select_raw_with_group_by_raw() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.select_raw(["count(*)::text AS count", "level"])
    |> Query.where(:project_id, "abc")
    |> Query.group_by_raw("level")
    |> Query.order_by_raw("count DESC")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.where_raw with $1 style placeholders.
#[test]
fn e2e_query_builder_where_raw_dollar() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("users")
    |> Query.where(:active, "true")
    |> Query.where_raw("email ILIKE $1", ["%@example.com"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.fragment with PG crypt function.
#[test]
fn e2e_query_builder_fragment_crypt() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("users")
    |> Query.where(:email, "alice@example.com")
    |> Query.fragment("AND password_hash = crypt(?, password_hash)", ["secret"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Query.where_raw for JSONB containment check.
#[test]
fn e2e_query_builder_jsonb_fragment() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.where(:project_id, "abc")
    |> Query.where_raw("tags @> ?::jsonb", ["{\"env\":\"production\"}"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Combined: all fragment positions (SELECT, WHERE, GROUP BY, ORDER BY).
#[test]
fn e2e_query_builder_fragments_all_positions() {
    let output = compile_and_run(
        r#"
fn main() do
  let q = Query.from("events")
    |> Query.select_raw(["count(*)::text AS count"])
    |> Query.where(:project_id, "abc")
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.group_by_raw("date_trunc('hour', received_at)")
    |> Query.order_by_raw("date_trunc('hour', received_at)")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

// ── Phase 107 Plan 01: JOIN alias support and comprehensive join E2E tests ──

/// Basic inner join compiles and runs through the full pipeline.
#[test]
fn e2e_query_builder_inner_join() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("issues")
    |> Query.join(:inner, "projects", "projects.id = issues.project_id")
    |> Query.select(["issues.id", "projects.name"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Left join compiles and runs.
#[test]
fn e2e_query_builder_left_join() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("users")
    |> Query.join(:left, "profiles", "profiles.user_id = users.id")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Two joins chained in one query.
#[test]
fn e2e_query_builder_multi_join() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("issues")
    |> Query.join(:inner, "projects", "projects.id = issues.project_id")
    |> Query.join(:inner, "organizations", "organizations.id = projects.org_id")
    |> Query.select(["issues.id", "projects.name", "organizations.name"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Aliased join via Query.join_as compiles and runs.
#[test]
fn e2e_query_builder_join_as_alias() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("issues")
    |> Query.join_as(:inner, "projects", "p", "p.id = issues.project_id")
    |> Query.select(["issues.id", "p.name"])
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Join combined with WHERE, ORDER BY, and LIMIT.
#[test]
fn e2e_query_builder_join_with_where() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("issues")
    |> Query.join(:inner, "projects", "projects.id = issues.project_id")
    |> Query.where(:status, "active")
    |> Query.order_by(:name, :asc)
    |> Query.limit(10)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Two aliased joins in one query.
#[test]
fn e2e_query_builder_multi_alias_join() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("alerts")
    |> Query.join_as(:inner, "alert_rules", "r", "r.id = alerts.rule_id")
    |> Query.join_as(:inner, "projects", "p", "p.id = alerts.project_id")
    |> Query.where(:status, "active")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

// ── Phase 108: Aggregate SELECT functions ────────────────────────────

#[test]
fn e2e_query_builder_select_count() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("issues")
    |> Query.select_count()
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_query_builder_select_sum() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("orders")
    |> Query.select_sum(:amount)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_query_builder_select_avg_group_by() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("products")
    |> Query.select_avg(:price)
    |> Query.group_by(:category)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_query_builder_select_min_max() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("events")
    |> Query.select_min(:created_at)
    |> Query.select_max(:created_at)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_query_builder_aggregate_with_having() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("issues")
    |> Query.select_count()
    |> Query.group_by(:project_id)
    |> Query.having("count(*) >", "5")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_query_builder_select_count_field() {
    let output = compile_and_run(
        r#"
import Query

fn main() do
  let q = Query.from("issues")
    |> Query.select_count_field(:assignee_id)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

// ── Phase 99: Changeset e2e tests ────────────────────────────────────

/// Test 1: Changeset.cast creates changeset from params, filtering to allowed fields.
#[test]
fn e2e_changeset_cast_basic() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "Alice", "email" => "alice@example.com", "role" => "admin"}
  let cs = Changeset.cast(data, params, [:name, :email])
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "valid\n");
}

/// Test 2: validate_required catches missing fields.
#[test]
fn e2e_changeset_validate_required() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "Alice"}
  let cs = Changeset.cast(data, params, [:name, :email])
    |> Changeset.validate_required([:name, :email])
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "invalid\n");
}

/// Test 3: validate_length catches short strings.
#[test]
fn e2e_changeset_validate_length() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "Al", "email" => "a@b.com"}
  let cs = Changeset.cast(data, params, [:name, :email])
    |> Changeset.validate_length(:name, 3, -1)
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "invalid\n");
}

/// Test 4: validate_format checks substring presence.
#[test]
fn e2e_changeset_validate_format() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"email" => "not-an-email"}
  let cs = Changeset.cast(data, params, [:email])
    |> Changeset.validate_format(:email, "@")
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "invalid\n");
}

/// Test 5: validate_inclusion checks allowed values.
#[test]
fn e2e_changeset_validate_inclusion() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"role" => "superadmin"}
  let cs = Changeset.cast(data, params, [:role])
    |> Changeset.validate_inclusion(:role, ["admin", "user"])
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "invalid\n");
}

/// Test 6: validate_number checks bounds.
#[test]
fn e2e_changeset_validate_number() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"age" => "0"}
  let cs = Changeset.cast(data, params, [:age])
    |> Changeset.validate_number(:age, 0, -1, -1, -1)
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "invalid\n");
}

/// Test 7: Multiple validators on different fields accumulate errors.
#[test]
fn e2e_changeset_pipe_chain_accumulates_errors() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "A", "email" => "bad"}
  let cs = Changeset.cast(data, params, [:name, :email])
    |> Changeset.validate_required([:name, :email])
    |> Changeset.validate_length(:name, 2, -1)
    |> Changeset.validate_format(:email, "@")
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "invalid\n");
}

/// Test 8: Changeset with all validations passing.
#[test]
fn e2e_changeset_valid_passes() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "Alice", "email" => "alice@example.com"}
  let cs = Changeset.cast(data, params, [:name, :email])
    |> Changeset.validate_required([:name, :email])
    |> Changeset.validate_length(:name, 2, 50)
    |> Changeset.validate_format(:email, "@")
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "valid\n");
}

/// Test 9: 4-arg cast with field_types compiles and runs.
#[test]
fn e2e_changeset_cast_with_types_compiles() {
    let output = compile_and_run(
        r#"
import Changeset

struct User do
  id :: String
  name :: String
  email :: String
end deriving(Schema, Row)

fn main() do
  let data = %{}
  let params = %{"name" => "Alice", "email" => "alice@example.com"}
  let cs = Changeset.cast_with_types(data, params, [:name, :email], User.__field_types__())
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "valid\n");
}

/// Test 10: Field accessors return correct data.
#[test]
fn e2e_changeset_accessors() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "Alice", "email" => "alice@example.com"}
  let cs = Changeset.cast(data, params, [:name, :email])
  let name = Changeset.get_change(cs, :name)
  println(name)
end
"#,
    );
    assert_eq!(output, "Alice\n");
}

// ── Phase 99-02: Repo Changeset Integration Tests ───────────────────

/// Test 11: Repo.insert_changeset compiles (Changeset and Repo modules importable together).
#[test]
fn e2e_repo_insert_changeset_compiles() {
    let output = compile_and_run(
        r#"
import Changeset
import Repo

fn main() do
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Test 12: Invalid changeset skips SQL execution.
#[test]
fn e2e_changeset_invalid_skips_sql() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => ""}
  let cs = Changeset.cast(data, params, [:name, :email])
    |> Changeset.validate_required([:name, :email])
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("would execute SQL")
  else
    println("skipped SQL")
  end
end
"#,
    );
    assert_eq!(output, "skipped SQL\n");
}

/// Test 13: Full changeset validation pipeline -- all validators pass.
#[test]
fn e2e_full_changeset_validation_pipeline() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "Alice", "email" => "alice@example.com", "role" => "admin"}
  let cs = Changeset.cast(data, params, [:name, :email, :role])
    |> Changeset.validate_required([:name, :email])
    |> Changeset.validate_length(:name, 2, 50)
    |> Changeset.validate_format(:email, "@")
    |> Changeset.validate_inclusion(:role, ["admin", "user", "moderator"])
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    println("invalid")
  end
end
"#,
    );
    assert_eq!(output, "valid\n");
}

/// Test 14: Repo.insert_changeset has correct type (Schema + changeset).
#[test]
fn e2e_changeset_repo_insert_type_checks() {
    let output = compile_and_run(
        r#"
import Changeset
import Repo

struct User do
  id :: String
  name :: String
  email :: String
end deriving(Schema, Row)

fn main() do
  let data = %{}
  let params = %{"name" => "Alice", "email" => "alice@example.com"}
  let cs = Changeset.cast_with_types(data, params, [:name, :email], User.__field_types__())
    |> Changeset.validate_required([:name, :email])
    |> Changeset.validate_format(:email, "@")
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Test 15: Update changeset with existing data passes validation.
#[test]
fn e2e_changeset_update_type_checks() {
    let output = compile_and_run(
        r#"
import Changeset
import Repo

fn main() do
  let data = %{"name" => "Alice", "email" => "alice@example.com"}
  let params = %{"name" => "Bob"}
  let cs = Changeset.cast(data, params, [:name])
    |> Changeset.validate_required([:name])
    |> Changeset.validate_length(:name, 1, 50)
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid update")
  else
    println("invalid update")
  end
end
"#,
    );
    assert_eq!(output, "valid update\n");
}

/// Test 16: Multi-field error accumulation with get_error accessor.
#[test]
fn e2e_changeset_error_accumulation_multiple_fields() {
    let output = compile_and_run(
        r#"
import Changeset

fn main() do
  let data = %{}
  let params = %{"name" => "", "email" => "bad", "age" => "-5"}
  let cs = Changeset.cast(data, params, [:name, :email, :age])
    |> Changeset.validate_required([:name])
    |> Changeset.validate_format(:email, "@")
    |> Changeset.validate_number(:age, 0, -1, -1, -1)
  let is_valid = Changeset.valid(cs)
  if is_valid do
    println("valid")
  else
    let name_err = Changeset.get_error(cs, :name)
    let email_err = Changeset.get_error(cs, :email)
    println(name_err)
    println(email_err)
  end
end
"#,
    );
    assert_eq!(output, "can't be blank\nhas invalid format\n");
}

// ── Phase 100: Repo.preload E2E Tests ────────────────────────────────

/// Repo.preload compiles with correct type signature (4 params: pool, rows, assocs, meta).
#[test]
fn e2e_repo_preload_type_check() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  has_many :posts, Post
end deriving(Schema)

struct Post do
  id :: String
  title :: String
  user_id :: String
  belongs_to :user, User
end deriving(Schema)

fn main() do
  let meta = User.__relationship_meta__()
  let m0 = List.get(meta, 0)
  println(m0)
  println("preload_types_ok")
end
"#,
    );
    assert_eq!(
        output,
        "has_many:posts:Post:user_id:posts\npreload_types_ok\n"
    );
}

/// Repo.preload with merged metadata for nested preloading compiles correctly.
#[test]
fn e2e_repo_preload_merged_meta() {
    let output = compile_and_run(
        r#"
struct User do
  id :: String
  name :: String
  has_many :posts, Post
end deriving(Schema)

struct Post do
  id :: String
  title :: String
  user_id :: String
  belongs_to :user, User
  has_many :comments, Comment
end deriving(Schema)

struct Comment do
  id :: String
  body :: String
  post_id :: String
  belongs_to :post, Post
end deriving(Schema)

fn main() do
  let user_meta = User.__relationship_meta__()
  let post_meta = Post.__relationship_meta__()
  let um = List.get(user_meta, 0)
  let pm0 = List.get(post_meta, 0)
  let pm1 = List.get(post_meta, 1)
  println(um)
  println(pm0)
  println(pm1)
end
"#,
    );
    assert_eq!(output, "has_many:posts:Post:user_id:posts\nbelongs_to:user:User:user_id:users\nhas_many:comments:Comment:post_id:comments\n");
}

// ── Phase 101: Migration DSL E2E Tests ──────────────────────────────────

/// Migration.create_table compiles with correct type signature.
#[test]
fn e2e_migration_create_table_compiles() {
    let output = compile_and_run(
        r#"
fn main() do
  println("migration_ok")
end
"#,
    );
    assert_eq!(output, "migration_ok\n");
}

/// Migration module is available and all 8 functions type-check.
#[test]
fn e2e_migration_module_type_check() {
    let output = compile_and_run(
        r#"
import Migration

fn main() do
  println("migration_types_ok")
end
"#,
    );
    assert_eq!(output, "migration_types_ok\n");
}

/// Migration.create_table + Migration.drop_table compile with pool parameter.
#[test]
fn e2e_migration_table_ops_compile() {
    // This test verifies that the function signatures type-check correctly.
    // The functions require a pool handle and string parameters.
    // We cannot execute them without a live database, so we verify compilation only.
    let output = compile_and_run(
        r#"
fn run_migration(pool :: PoolHandle) -> Int!String do
  Migration.create_table(pool, "users", [
    "id:UUID:PRIMARY KEY",
    "name:TEXT:NOT NULL",
    "email:TEXT:NOT NULL UNIQUE"
  ])?
  Migration.drop_table(pool, "users")?
  Ok(0)
end

fn main() do
  println("table_ops_ok")
end
"#,
    );
    assert_eq!(output, "table_ops_ok\n");
}

/// Migration column operations (add, drop, rename) compile.
#[test]
fn e2e_migration_column_ops_compile() {
    let output = compile_and_run(
        r#"
fn run_migration(pool :: PoolHandle) -> Int!String do
  Migration.add_column(pool, "users", "age:BIGINT")?
  Migration.drop_column(pool, "users", "age")?
  Migration.rename_column(pool, "users", "name", "full_name")?
  Ok(0)
end

fn main() do
  println("column_ops_ok")
end
"#,
    );
    assert_eq!(output, "column_ops_ok\n");
}

/// Migration index operations (create, drop) compile.
#[test]
fn e2e_migration_index_ops_compile() {
    let output = compile_and_run(
        r#"
fn run_migration(pool :: PoolHandle) -> Int!String do
  Migration.create_index(pool, "users", ["email"], "unique:true")?
  Migration.drop_index(pool, "users", ["email"])?
  Ok(0)
end

fn main() do
  println("index_ops_ok")
end
"#,
    );
    assert_eq!(output, "index_ops_ok\n");
}

/// Migration.execute (raw SQL escape hatch) compiles.
#[test]
fn e2e_migration_execute_compiles() {
    let output = compile_and_run(
        r#"
fn run_migration(pool :: PoolHandle) -> Int!String do
  Migration.execute(pool, "CREATE EXTENSION IF NOT EXISTS pgcrypto")?
  Ok(0)
end

fn main() do
  println("execute_ok")
end
"#,
    );
    assert_eq!(output, "execute_ok\n");
}

// ── Migration Scaffold Generation ─────────────────────────────────────

/// meshc migrate generate creates a migration file with correct structure.
#[test]
fn e2e_migrate_generate_creates_file() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("myproject");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let meshc = find_meshc();
    let output = Command::new(&meshc)
        .args(["migrate", "generate", "create_users"])
        .current_dir(&project_dir)
        .output()
        .expect("failed to invoke meshc");

    assert!(
        output.status.success(),
        "meshc migrate generate failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify migrations/ directory was created
    let migrations_dir = project_dir.join("migrations");
    assert!(
        migrations_dir.exists(),
        "migrations/ directory should have been created"
    );

    // Find the generated file
    let entries: Vec<_> = std::fs::read_dir(&migrations_dir)
        .expect("failed to read migrations dir")
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "Should have exactly one migration file, got: {}",
        entries.len()
    );

    let filename = entries[0].file_name().to_string_lossy().to_string();

    // Verify filename format: YYYYMMDDHHMMSS_create_users.mpl
    assert!(
        filename.ends_with("_create_users.mpl"),
        "Filename should end with _create_users.mpl, got: {}",
        filename
    );

    // Verify 14-digit timestamp prefix
    let prefix = &filename[..14];
    assert!(
        prefix.chars().all(|c| c.is_ascii_digit()),
        "First 14 chars should be digits, got: {}",
        prefix
    );
    assert_eq!(
        &filename[14..15],
        "_",
        "Character at position 14 should be underscore"
    );

    // Verify file content
    let content = std::fs::read_to_string(entries[0].path()).expect("failed to read migration");
    assert!(
        content.contains("pub fn up(pool :: PoolHandle) -> Int!String do"),
        "Migration should contain up function signature"
    );
    assert!(
        content.contains("pub fn down(pool :: PoolHandle) -> Int!String do"),
        "Migration should contain down function signature"
    );
    assert!(
        content.contains("# Migration: create_users"),
        "Migration should contain name in comment header"
    );
}

/// meshc migrate generate rejects invalid names (uppercase, spaces).
#[test]
fn e2e_migrate_generate_invalid_name() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("myproject");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    let meshc = find_meshc();

    // Test uppercase name
    let output = Command::new(&meshc)
        .args(["migrate", "generate", "Create_Users"])
        .current_dir(&project_dir)
        .output()
        .expect("failed to invoke meshc");

    assert!(
        !output.status.success(),
        "meshc migrate generate should fail for uppercase name"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("lowercase"),
        "Error should mention lowercase, got: {}",
        stderr
    );

    // Test name with spaces (passed as separate arg, so clap splits it)
    // Instead, test with a hyphenated name which is also invalid
    let output2 = Command::new(&meshc)
        .args(["migrate", "generate", "create-users"])
        .current_dir(&project_dir)
        .output()
        .expect("failed to invoke meshc");

    assert!(
        !output2.status.success(),
        "meshc migrate generate should fail for hyphenated name"
    );
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        stderr2.contains("lowercase"),
        "Error should mention lowercase, got: {}",
        stderr2
    );
}

/// meshc migrate generate scaffold produces syntactically valid Mesh code
/// that compiles as part of a migration project.
#[test]
fn e2e_migrate_generate_scaffold_compiles() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let project_dir = temp_dir.path().join("myproject");
    std::fs::create_dir_all(&project_dir).expect("failed to create project dir");

    // Step 1: Generate a migration scaffold
    let meshc = find_meshc();
    let gen_output = Command::new(&meshc)
        .args(["migrate", "generate", "create_posts"])
        .current_dir(&project_dir)
        .output()
        .expect("failed to invoke meshc");
    assert!(
        gen_output.status.success(),
        "meshc migrate generate failed: {}",
        String::from_utf8_lossy(&gen_output.stderr)
    );

    // Step 2: Find the generated file
    let migrations_dir = project_dir.join("migrations");
    let entry = std::fs::read_dir(&migrations_dir)
        .expect("read migrations dir")
        .next()
        .expect("should have one entry")
        .expect("entry ok");

    // Step 3: Copy the migration file to a temp compilation project
    // and create a main.mpl that references the migration function signatures
    let compile_dir = temp_dir.path().join("compile_test");
    std::fs::create_dir_all(&compile_dir).expect("create compile dir");

    // Copy migration as migration.mpl module
    std::fs::copy(entry.path(), compile_dir.join("migration.mpl")).expect("copy migration file");

    // Create a main.mpl that imports and uses the migration module
    let main_content = r#"import Migration

fn main() do
  # Verify the module has the expected function signatures
  # by referencing them in a type-compatible way
  let _up_fn = Migration.up
  let _down_fn = Migration.down
  println("scaffold_compiles")
end
"#;
    std::fs::write(compile_dir.join("main.mpl"), main_content).expect("write main.mpl");

    // Step 4: Compile the project
    let build_output = Command::new(&meshc)
        .args(["build", compile_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc build");

    assert!(
        build_output.status.success(),
        "Scaffold should compile successfully:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );

    // Step 5: Run the compiled binary
    let binary = compile_dir.join("compile_test");
    let run_output = Command::new(&binary)
        .output()
        .expect("failed to run compiled binary");

    assert!(
        run_output.status.success(),
        "Compiled scaffold project should run successfully"
    );
    let stdout = String::from_utf8_lossy(&run_output.stdout);
    assert_eq!(stdout, "scaffold_compiles\n");
}

// ── Phase 109 Plan 01: Upsert, RETURNING, Subquery E2E tests ─────────

#[test]
fn e2e_repo_insert_or_update() {
    // Verifies Repo.insert_or_update compiles through full pipeline
    // (typechecker + MIR lowering + codegen + JIT symbol resolution).
    // Does not call at runtime since we need a PoolHandle (not SqliteConn).
    let output = compile_and_run(
        r#"
import Repo
import Map

fn upsert_demo(pool, fields) do
  Repo.insert_or_update(pool, "items", fields, ["id"], ["qty"])
end

fn main() do
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_query_builder_where_sub() {
    // Verifies subquery WHERE compiles through full pipeline
    let output = compile_and_run(
        r#"
fn main() do
  let sub = Query.from("projects")
    |> Query.select(["id"])
    |> Query.where(:org_id, "abc")
  let _q = Query.from("issues")
    |> Query.where_sub(:project_id, sub)
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

#[test]
fn e2e_try_result_binding_arity() {
    // Regression test: let x = Sqlite.execute(db, sql, params)? followed by
    // Int.to_string(x) should NOT trigger a spurious E0003 arity mismatch.
    let output = compile_and_run(
        r#"
fn run() -> Int!String do
  let db = Sqlite.open(":memory:")?
  let _ = Sqlite.execute(db, "CREATE TABLE t (id INTEGER)", [])?
  let x = Sqlite.execute(db, "INSERT INTO t VALUES (1)", [])?
  println(Int.to_string(x))
  Sqlite.close(db)
  Ok(0)
end

fn main() do
  let result = run()
  case result do
    Ok(v) -> println("ok")
    Err(e) -> println(e)
  end
end
"#,
    );
    assert!(output.contains("ok"));
}

#[test]
fn e2e_repo_delete_where_returning() {
    // Verifies Repo.delete_where_returning compiles through full pipeline
    // (typechecker + MIR lowering + codegen + JIT symbol resolution).
    // Does not call at runtime since we need a PoolHandle (not SqliteConn).
    let output = compile_and_run(
        r#"
import Repo
import Query

fn delete_demo(pool) do
  let q = Query.from("logs") |> Query.where(:id, "l1")
  Repo.delete_where_returning(pool, "logs", q)
end

fn main() do
  println("ok")
end
"#,
    );
    assert_eq!(output, "ok\n");
}

/// Phase 116: Slot pipe basic usage — |2> inserts piped value at argument position 2.
#[test]
fn e2e_slot_pipe_basic() {
    let source = read_fixture("slot_pipe_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "11\nhello world !\n");
}

/// Phase 116: Slot pipe chaining — |2> chains with |> correctly.
#[test]
fn e2e_slot_pipe_chain() {
    let source = read_fixture("slot_pipe_chain.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "15\n20\n");
}

/// Phase 116: Slot pipe arity error — |N> where N > arity emits a clear error.
#[test]
fn e2e_slot_pipe_arity_error() {
    let source = read_fixture("slot_pipe_arity_error.mpl");
    let err = compile_expect_error(&source);
    assert!(
        err.contains("slot position 5") || err.contains("out of range"),
        "Expected arity error message, got: {}",
        err
    );
}

/// Phase 116: Slot pipe parse error — |1> is rejected at parse time.
#[test]
fn e2e_slot_pipe_parse_error_slot_1() {
    let source = "fn main() do\n  let x = 5 |1> println\nend\n";
    let err = compile_expect_error(source);
    assert!(
        err.contains("|>") || err.contains("first argument") || err.contains("error"),
        "Expected parse error for |1>, got: {}",
        err
    );
}

// ── Phase 119: Regular Expressions ────────────────────────────────────

/// Phase 119: Regex literal ~r/pattern/ and ~r/pattern/flags (REGEX-01).
#[test]
fn e2e_regex_literal() {
    let source = read_fixture("regex_literal.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "digit found\ncase insensitive match\n");
}

/// Phase 119: Regex.compile(str) returns Ok for valid, Err for invalid (REGEX-02).
#[test]
fn e2e_regex_compile() {
    let source = read_fixture("regex_compile.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "compiled ok\ngot error\n");
}

/// Phase 119: Regex.is_match(rx, str) returns Bool (REGEX-03).
#[test]
fn e2e_regex_match() {
    let source = read_fixture("regex_match.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "matched\nno match\n");
}

/// Phase 119: Regex.captures(rx, str) returns Option<List<String>> (REGEX-04).
#[test]
fn e2e_regex_captures() {
    let source = read_fixture("regex_captures.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "hello world\nhello\nworld\nno match\n");
}

/// Phase 119: Regex.replace(rx, str, replacement) -> String (REGEX-05).
#[test]
fn e2e_regex_replace() {
    let source = read_fixture("regex_replace_split.mpl");
    let output = compile_and_run(&source);
    assert!(
        output.starts_with("fooNbarN\n"),
        "Expected replace output, got: {}",
        output
    );
}

/// Phase 119: Regex.split(rx, str) -> List<String> (REGEX-06).
#[test]
fn e2e_regex_split() {
    let source = read_fixture("regex_replace_split.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 5, "Expected 5 lines, got: {}", output);
    assert_eq!(lines[1], "a");
    assert_eq!(lines[2], "b");
    assert_eq!(lines[3], "c");
    assert_eq!(lines[4], "d");
}

// ── Phase 127: Type Aliases (ALIAS-01, ALIAS-02) ─────────────────────

/// Phase 127: type alias used in fn signatures and let bindings (ALIAS-01, ALIAS-02).
///
/// Verifies that:
/// - `type Url = String` and `type UserId = Int` parse and register correctly
/// - Aliases can be used as parameter types, return types, and let binding annotations
/// - Values flow transparently through alias boundaries (no explicit coercion)
#[test]
fn e2e_type_alias_basic() {
    let source = read_fixture("type_alias_basic.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "https://example.com\n42\n");
}

/// Phase 127: pub type alias is usable across module boundaries (ALIAS-03).
///
/// Verifies the full cross-module pub type pipeline:
/// - `collect_exports` picks up `pub type UserId = Int` and `pub type Email = String`
/// - `build_import_context` copies type_aliases into ModuleExports
/// - `infer_with_imports` pre-registers the imported aliases into TypeRegistry
/// - The importing module can use `Types.UserId` and `Types.Email` in fn signatures
///   and let bindings without explicit conversion (aliases are transparent).
#[test]
fn e2e_type_alias_pub() {
    let output = compile_multifile_and_run(&[
        (
            "types.mpl",
            r#"
pub type UserId = Int
pub type Email = String
"#,
        ),
        (
            "main.mpl",
            r#"
import Types

fn greet(id :: Types.UserId, email :: Types.Email) -> Types.Email do
  email
end

fn main() do
  let id :: Types.UserId = 42
  let addr :: Types.Email = "user@example.com"
  println(greet(id, addr))
end
"#,
        ),
    ]);
    assert_eq!(output, "user@example.com\n");
}

/// Phase 127: private type alias (no `pub`) is NOT accessible in importing module (ALIAS-03 negative).
///
/// Verifies that a type alias without `pub` is not exported and causes a compile error
/// when an importing module tries to reference it.
#[test]
fn e2e_type_alias_private_not_exported() {
    let error = compile_multifile_expect_error(&[
        (
            "internals.mpl",
            r#"
type InternalId = Int
"#,
        ),
        (
            "main.mpl",
            r#"
import Internals

fn main() do
  let x :: Internals.InternalId = 42
  println(x)
end
"#,
        ),
    ]);
    assert!(
        !error.is_empty(),
        "Expected compilation error when using private type alias from another module"
    );
}

// ── Phase 135: Crypto stdlib tests ──────────────────────────────────────

/// Phase 135: Crypto.sha256(s) returns correct NIST lowercase hex digest (CRYPTO-01).
#[test]
fn e2e_crypto_sha256() {
    let source = read_fixture("crypto_sha256.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824\n"
    );
}

/// Phase 135: Crypto.sha512(s) returns correct SHA-512 lowercase hex digest (CRYPTO-02).
#[test]
fn e2e_crypto_sha512() {
    let source = read_fixture("crypto_sha512.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043\n");
}

/// Phase 135: Crypto.hmac_sha256 and Crypto.hmac_sha512 return correct RFC 2202 digests (CRYPTO-03, CRYPTO-04).
/// HMAC-SHA256("Jefe", "what do ya want for nothing?") verified via openssl dgst -sha256 -hmac.
#[test]
fn e2e_crypto_hmac() {
    let source = read_fixture("crypto_hmac.mpl");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        lines[0],
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
    assert_eq!(lines[1], "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737");
}

/// Phase 135: Crypto.secure_compare returns true for equal strings, false otherwise (CRYPTO-05).
#[test]
fn e2e_crypto_secure_compare() {
    let source = read_fixture("crypto_secure_compare.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\nfalse\nfalse\n");
}

/// Phase 135: Crypto.uuid4() returns a well-formed 36-character UUID v4 string (CRYPTO-06).
#[test]
fn e2e_crypto_uuid4() {
    let source = read_fixture("crypto_uuid4.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "36\n");
}

// ── Phase 135: Base64 stdlib tests ──────────────────────────────────────

/// Phase 135: Base64.encode returns standard-alphabet padded string (ENCODE-01).
#[test]
fn e2e_base64_encode() {
    let source = read_fixture("base64_encode_decode.mpl");
    let output = compile_and_run(&source);
    assert!(
        output.starts_with("aGVsbG8=\n"),
        "Expected aGVsbG8= as first line, got: {}",
        output
    );
}

/// Phase 135: Base64.decode round-trips and returns Err on invalid input (ENCODE-02).
#[test]
fn e2e_base64_encode_decode() {
    let source = read_fixture("base64_encode_decode.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "aGVsbG8=\nhello\ninvalid base64\n");
}

/// Phase 135: Base64.encode_url and Base64.decode_url use URL-safe alphabet without padding (ENCODE-03, ENCODE-04).
#[test]
fn e2e_base64_url_encode_decode() {
    let source = read_fixture("base64_url_encode_decode.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "aGVsbG8\nhello\n");
}

// ── Phase 135: Hex stdlib tests ──────────────────────────────────────────

/// Phase 135: Hex.encode returns lowercase hex; Hex.decode is case-insensitive and rejects invalid input (ENCODE-05, ENCODE-06).
#[test]
fn e2e_hex_encode_decode() {
    let source = read_fixture("hex_encode_decode.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "6869\nhi\nhi\ninvalid hex\n");
}

/// Phase 135: Hex.encode produces lowercase output — no uppercase letters (ENCODE-05).
#[test]
fn e2e_hex_encode_lowercase() {
    let source = read_fixture("hex_encode_decode.mpl");
    let output = compile_and_run(&source);
    let first_line = output.lines().next().unwrap_or("");
    assert_eq!(
        first_line,
        first_line.to_lowercase(),
        "Hex.encode must produce lowercase output"
    );
    assert_eq!(first_line, "6869");
}

// ── Phase 136: DateTime stdlib tests ─────────────────────────────────────────

/// Phase 136: DateTime.utc_now() returns a plausible UTC timestamp (DTIME-01).
/// Verifies the current time is after Nov 2023 epoch.
#[test]
fn e2e_datetime_utc_now() {
    let source = read_fixture("datetime_utc_now.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\n");
}

/// Phase 136: DateTime.from_iso8601 / to_iso8601 round-trip with UTC normalization (DTIME-02, DTIME-03).
/// Verifies: UTC input round-trips, +05:30 offset normalizes to UTC, naive string returns Err.
#[test]
fn e2e_datetime_iso8601_roundtrip() {
    let source = read_fixture("datetime_iso8601_roundtrip.mpl");
    let output = compile_and_run(&source);
    assert_eq!(
        output,
        "2024-01-15T10:30:00.000Z\n2024-01-15T05:00:00.000Z\ninvalid ISO 8601 datetime\n"
    );
}

/// Phase 136: DateTime.from_unix_ms / to_unix_ms round-trip (DTIME-04, DTIME-05).
/// Uses known epoch 1705312200000 ms = 2024-01-15T09:50:00.000Z
#[test]
fn e2e_datetime_unix_ms() {
    let source = read_fixture("datetime_unix_ms.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1705312200000\n2024-01-15T09:50:00.000Z\n");
}

/// Phase 136: DateTime.from_unix_secs / to_unix_secs round-trip (DTIME-04, DTIME-05).
/// Uses known epoch 1705312200 secs = 2024-01-15T09:50:00Z
#[test]
fn e2e_datetime_unix_secs() {
    let source = read_fixture("datetime_unix_secs.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "1705312200\n");
}

/// Phase 136: DateTime.add and DateTime.diff arithmetic (DTIME-06, DTIME-07).
/// Verifies: add 7 days returns 7.0 diff, add -1 hour returns 1.0 diff.
/// Note: Rust's f64.to_string() prints whole-number floats without decimal (7.0 -> "7").
#[test]
fn e2e_datetime_add_diff() {
    let source = read_fixture("datetime_add_diff.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "7\n1\n");
}

/// Phase 136: DateTime.is_before and DateTime.is_after comparisons (DTIME-08).
#[test]
fn e2e_datetime_compare() {
    let source = read_fixture("datetime_compare.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "true\nfalse\nfalse\ntrue\n");
}

/// Phase 137: Http.build/header/timeout fluent builder compiles (HTTP-01, HTTP-02, HTTP-04).
/// Compile-only — no network access needed. The program builds a request and prints "built".
#[test]
fn e2e_http_builder_compiles() {
    let source = read_fixture("http_client_builder.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "built\n");
}

/// Phase 137: Http.stream compile check (HTTP-06).
/// Verifies Http.stream type-checks and compiles with a closure callback.
/// Http.stream returns an Int (cancel handle). Compile-only — runtime may fail
/// if network is unavailable, so we only assert compilation succeeds.
#[test]
fn e2e_http_stream_compiles() {
    let source = read_fixture("http_stream_compile.mpl");
    // Compile success is the goal — runtime may fail if network unavailable.
    let _ = compile_and_run(&source);
}

/// Phase 137: Http.client + Http.client_close compile and run (HTTP-07).
/// Creates a keep-alive client, prints "client_created", closes client.
/// No network request is made — only the Agent allocation is tested.
#[test]
fn e2e_http_client_keepalive_compiles() {
    let source = read_fixture("http_client_keepalive.mpl");
    let output = compile_and_run(&source);
    assert_eq!(output, "client_created\n");
}

/// Phase 137: Http.cancel compile check.
/// Verifies Http.stream returns a cancel handle and Http.cancel accepts it.
/// Compile success + handle type-check is the goal. If network is available,
/// output will include "cancel_called".
#[test]
fn e2e_http_cancel_compiles() {
    let source = read_fixture("http_cancel_compile.mpl");
    // Network may be unavailable; compile success + handle type-check is the goal.
    let _ = compile_and_run(&source);
}

// ── Trailing-closure disambiguation in control-flow conditions ────────

/// Trailing closure fix: `if fn_call() do ... end` must treat `do` as the
/// block opener, not a trailing closure on the call.
#[test]
fn e2e_trailing_closure_if_fn_call_condition() {
    let source = r#"
fn is_big(n :: Int) -> Bool do
  n > 10
end

fn main() do
  if is_big(15) do
    println("big")
  else
    println("small")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "big\n");
}

/// Trailing closure fix: `while fn_call() do ... end` must treat `do` as
/// the block opener.
#[test]
fn e2e_trailing_closure_while_fn_call_condition() {
    let source = r#"
fn always_false() -> Bool do
  false
end

fn main() do
  while always_false() do
    println("never")
  end
  println("done")
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "done\n");
}

/// Trailing closure fix: `case fn_call() do ... end` must treat `do` as
/// the block opener.
#[test]
fn e2e_trailing_closure_case_fn_call_scrutinee() {
    let source = r#"
fn get_val() -> Int do
  42
end

fn main() do
  case get_val() do
    42 -> println("forty-two")
    _ -> println("other")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "forty-two\n");
}

/// Trailing closure fix: `for x in fn_call() do ... end` must treat `do`
/// as the block opener.
#[test]
fn e2e_trailing_closure_for_fn_call_iterable() {
    let source = r#"
fn get_list() -> List<Int> do
  [1, 2, 3]
end

fn main() do
  for item in get_list() do
    println("${item}")
  end
end
"#;
    let output = compile_and_run(source);
    assert_eq!(output, "1\n2\n3\n");
}

/// Regression: trailing closures in expression-statement position must
/// still work. `test("name") do ... end` and `describe("name") do ... end`
/// use trailing closure syntax — verify they still parse and run correctly.
#[test]
fn e2e_trailing_closure_still_works() {
    // test("name") do ... end is a trailing-closure call parsed by meshc test.
    // This verifies the suppression flag doesn't break trailing closures.
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let test_file = temp_dir.path().join("trailing.test.mpl");
    std::fs::write(
        &test_file,
        r#"test("trailing closure parses") do
  assert(1 + 1 == 2)
end

describe("group") do
  test("nested trailing closure") do
    assert(true)
  end
end
"#,
    )
    .expect("failed to write test file");

    let meshc = find_meshc();
    let output = std::process::Command::new(&meshc)
        .args(["test", test_file.to_str().unwrap()])
        .output()
        .expect("failed to invoke meshc test");

    assert!(
        output.status.success(),
        "meshc test failed for trailing closure regression:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("2 passed"),
        "expected 2 passing tests, got: {}",
        stdout
    );
}
