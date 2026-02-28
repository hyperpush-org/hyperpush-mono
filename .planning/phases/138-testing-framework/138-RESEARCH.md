# Phase 138: Testing Framework - Research

**Researched:** 2026-02-28
**Domain:** Mesh compiler extension — test DSL, runner CLI, stdlib assertion functions, mock actor support
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Test output style**
- Default mode is verbose: each test printed by name as it runs (`✓ User.greet returns hello` / `✗ User.greet handles nil`)
- `--quiet` flag switches to compact dots mode (`.` for pass, `F` for fail)
- Colors always enabled (green ✓, red ✗, red-highlighted values in failure messages) — no TTY detection
- Failures print inline as they happen (expression + expected/actual values), then a summary of all failures is reprinted at the bottom after the run
- Final summary line format: `3 passed, 1 failed in 0.42s`

**meshc test CLI options**
- File path filter: `meshc test path/to/file.test.mpl` runs only that file; no name-based filtering
- Test file discovery: recursive from project root, finds all `*.test.mpl` files
- `--coverage` flag: accepted, prints "Coverage reporting coming soon" and exits cleanly
- No `--watch` mode in this phase

**Test file conventions**
- Every test must be inside a named `test "description" do ... end` block — no bare top-level assertions
- `test` blocks can appear at the top level of a file OR inside a `describe` block; both are valid
- `describe "..." do ... end` groups tests: group name prefixes test name in output
- No header or import needed — `.test.mpl` extension signals the compiler to enable the test DSL
- `setup do ... end` and `teardown do ... end` inside a `describe` run before/after **each** test in that group (per-test scope, not once-per-describe)

**Mock actors and assert_receive**
- `Test.mock_actor(fn msg -> ... end)` creates an isolated actor; unhandled messages are silently ignored
- Test isolation: before each test, named actors registered during the previous test are killed/unregistered — clean actor registry per test
- `assert_receive pattern, timeout_ms` uses Mesh pattern syntax (wildcards, tuples, etc.), consistent with the rest of the language
- Default timeout when omitted: 100ms; override with explicit second argument
- Timeout failure reports the pattern and elapsed time

### Claude's Discretion
- Exact ANSI color codes and formatting details
- Internal mechanism for per-test actor registry cleanup
- How `assert_raises fn` reports the raised value vs expected
- Exact progress output during compilation phase before tests run

### Deferred Ideas (OUT OF SCOPE)
- `--watch` mode (re-run tests on file change) — future phase
- Coverage reporting beyond stub — future phase
- `setup_all` / `teardown_all` (once per describe block) — could be added later if needed
- Name-based test filtering (`--name "greet"`) — future enhancement
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TEST-01 | User can run all `*.test.mpl` files in a project via `meshc test` with a pass/fail summary per test function | `meshc test` subcommand + runner compiles each file independently using existing `build()` pipeline, executes binary, captures exit code |
| TEST-02 | User can assert a boolean expression via `assert expr` with failure output showing expression source and value | `assert` as test-DSL builtin, lowered to `mesh_test_assert(cond, expr_src, file, line) -> void` |
| TEST-03 | User can assert equality via `assert_eq a, b` with expected vs actual output on failure | `assert_eq` as test-DSL builtin, lowered to `mesh_test_assert_eq_*` with type-dispatched string conversion |
| TEST-04 | User can assert inequality via `assert_ne a, b` with a descriptive failure message | `assert_ne` as test-DSL builtin, lowered to `mesh_test_assert_ne_*` |
| TEST-05 | User can assert that a function raises an error via `assert_raises fn` | `assert_raises` as test-DSL builtin wrapping closure in `catch_unwind`-equivalent; `mesh_test_assert_raises` |
| TEST-06 | User can group related tests via `describe "..." do ... end` blocks | Compiler-level construct — groups tests; output prefixes test name with group name |
| TEST-07 | User can define shared setup and teardown via `setup do` and `teardown do` inside `describe` | Compiler-level construct — run before/after each test in the group |
| TEST-08 | User can spawn a mock actor via `Test.mock_actor(fn msg -> ... end)` returning a Pid | `Test` registered as stdlib module; `mesh_test_mock_actor` runtime function |
| TEST-09 | User can assert the test actor receives a message via `assert_receive pattern, timeout` | `assert_receive` as test-DSL builtin; `mesh_test_assert_receive(timeout_ms) -> *const u8` + pattern match |
| TEST-10 | User can generate a test coverage report via `meshc test --coverage` | Stub implementation: `--coverage` flag accepted, prints "Coverage reporting coming soon", exits 0 |
</phase_requirements>

---

## Summary

Phase 138 builds the Mesh testing framework end-to-end: new `meshc test` subcommand, a test DSL recognized in `*.test.mpl` files, assertion builtins, describe/setup/teardown grouping, mock actor support, and a coverage stub.

The key architectural decision already locked in STATE.md is that each `*.test.mpl` is compiled and executed as a **complete standalone Mesh program**. The test runner does not inject function pointers or use reflection; instead, the compiler transforms `test "..." do ... end` blocks into a generated `main()` that runs each test, calls assertion builtins, and exits with code 1 if any assertion fails. The runner collects the per-file exit codes and assembles the summary.

The implementation spans the same 5 compiler registration points used by Crypto/DateTime/Http (builtins.rs, infer.rs `stdlib_modules()`, infer.rs `STDLIB_MODULE_NAMES`, lower.rs `STDLIB_MODULES` + `map_builtin_name` + `known_functions`, intrinsics.rs `declare_intrinsics`) plus a new Rust runtime module `compiler/mesh-rt/src/test.rs`. On top of that, the `meshc` CLI gets a new `Test` subcommand in `compiler/meshc/src/main.rs`, and a new `compiler/meshc/src/test_runner.rs` implements discovery, parallel compilation, and output formatting.

**Primary recommendation:** Follow the exact 5-point registration pattern established by Phase 135-137. The test DSL (`test`, `describe`, `setup`, `teardown`, `assert`, `assert_eq`, `assert_ne`, `assert_raises`, `assert_receive`) is handled at the MIR-lowering level as a code transformation step — not as new parser keywords — by recognizing special call patterns in `.test.mpl` files.

---

## Standard Stack

### Core

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| mesh-rt `test.rs` | (new) | `mesh_test_*` C ABI functions for assertion logic and mock actors | Same pattern as crypto.rs, datetime.rs |
| mesh-codegen lower.rs | existing | Register test functions in known_functions + STDLIB_MODULES | Established 5-point pattern |
| mesh-typeck infer.rs | existing | Type-check `Test.mock_actor`, `assert_*` calls | Same stdlib_modules() HashMap |
| meshc test_runner.rs | (new) | Discover `.test.mpl` files, compile+run each, aggregate output | Mirrors `build()` function structure |
| `std::panic::catch_unwind` | Rust std | Capture `mesh_panic` for `assert_raises` | Already used by actor crash isolation |

### Supporting

| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `tempfile` | (workspace dep, already used) | Temporary dirs for test compilation | Used already in e2e Rust tests |
| ANSI escape codes | n/a | Color output (green/red) | Always enabled per locked decision |
| `std::time::Instant` | Rust std | Timing the test run for summary line | For `N passed, M failed in X.XXs` |

### No New External Dependencies

The testing framework requires zero new Cargo dependencies. All needed capabilities (actor spawn/receive, GC allocation, panic, string creation) exist in mesh-rt already.

---

## Architecture Patterns

### Recommended Project Structure

```
compiler/
├── mesh-rt/src/test.rs          # NEW: mesh_test_* C ABI functions
├── mesh-rt/src/lib.rs           # ADD: pub mod test;
├── mesh-codegen/src/
│   ├── mir/lower.rs             # ADD: Test functions in known_functions, STDLIB_MODULES
│   └── codegen/intrinsics.rs   # ADD: declare mesh_test_* LLVM declarations
├── mesh-typeck/src/infer.rs     # ADD: Test module in stdlib_modules(), STDLIB_MODULE_NAMES
└── meshc/src/
    ├── main.rs                  # ADD: Test subcommand to clap Commands enum
    └── test_runner.rs           # NEW: discover, compile, run, format output
```

### Pattern 1: The 5-Point Registration

Every stdlib module follows this exact path. For the `Test` module:

**Point 1: `mesh-typeck/src/infer.rs` — `stdlib_modules()`**
```rust
// Source: existing pattern from Phase 135-137
let pid_t = Ty::untyped_pid();
let mut test_mod = HashMap::new();
// Test.mock_actor(fn(msg) -> String) -> Pid
let mock_cb_t = Ty::fun(vec![Ty::string()], Ty::string());
test_mod.insert("mock_actor".to_string(),
    Scheme::mono(Ty::fun(vec![mock_cb_t], pid_t.clone())));
modules.insert("Test".to_string(), test_mod);
```

**Point 2: `mesh-typeck/src/infer.rs` — `STDLIB_MODULE_NAMES`**
```rust
const STDLIB_MODULE_NAMES: &[&str] = &[
    // ... existing ...
    "Test",  // Phase 138
];
```

**Point 3: `mesh-codegen/src/mir/lower.rs` — `STDLIB_MODULES`**
```rust
const STDLIB_MODULES: &[&str] = &[
    // ... existing ...
    "Test",  // Phase 138
];
```

**Point 4: `mesh-codegen/src/mir/lower.rs` — `known_functions` + `map_builtin_name`**
```rust
// In register_known_functions():
self.known_functions.insert("mesh_test_mock_actor".to_string(),
    MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));

// In map_builtin_name():
"test_mock_actor" => "mesh_test_mock_actor".to_string(),
```

**Point 5: `mesh-codegen/src/codegen/intrinsics.rs` — `declare_intrinsics()`**
```rust
// mesh_test_mock_actor(fn_ptr: ptr, env_ptr: ptr) -> i64 (Pid)
let mock_actor_ty = i64_type.fn_type(
    &[ptr_type.into(), ptr_type.into()], false);
module.add_function("mesh_test_mock_actor", mock_actor_ty, External);
```

### Pattern 2: Test DSL — Compiler Transformation, Not New Keywords

The `test "desc" do ... end` and `describe "desc" do ... end` blocks are NOT new parser keywords. The locked decision is: `.test.mpl` extension signals test mode. The approach:

1. **In lower.rs**: Detect when lowering from a `.test.mpl` source (thread the file name through the lowering context).
2. **Recognize function call patterns**: `test("desc", fn() do ... end)` and `describe("desc", fn() do ... end)` as builtin calls.
3. **Generate a synthetic `main()`**: The lowerer collects all `test` blocks and `describe` blocks, generates the harness body that calls each test, calls setup/teardown, catches panics via the existing `catch_unwind` mechanism, and prints results.

The generated harness pseudocode (at MIR level):
```
fn main():
  mesh_rt_init_actor(1)        # start scheduler for mock actors
  let start = clock_now()
  # for each test block:
  mesh_test_begin("desc")
  [setup code if any]
  match catch_unwind(|| test_body()):
    Ok(_)  -> mesh_test_pass("desc")
    Err(e) -> mesh_test_fail("desc", e)
  [teardown code if any]
  mesh_test_summary(start)
  exit(fail_count > 0 ? 1 : 0)
```

Alternative simpler approach (RECOMMENDED for this phase): Treat `.test.mpl` files as normal Mesh programs where `test`, `describe`, `setup`, `teardown`, `assert`, `assert_eq`, `assert_ne`, `assert_raises`, `assert_receive` are registered as **builtin functions** (in `builtins.rs`). The transformation generates the harness at the MIR level. This avoids any parser changes.

### Pattern 3: assert_raises Using catch_unwind

The actor system already uses `std::panic::catch_unwind` to isolate actor crashes. The test runtime reuses this:

```rust
// Source: mesh-rt/src/actor/mod.rs — actor crash isolation pattern
#[no_mangle]
pub extern "C" fn mesh_test_assert_raises(
    fn_ptr: *const u8,
    env_ptr: *const u8,
    file: *const u8,
    file_len: u64,
    line: u32,
) {
    // Call the closure via the fn_ptr/env_ptr ABI (same as http_stream)
    // If it does NOT panic, report failure.
    // If it panics with mesh_panic, report success.
    let result = std::panic::catch_unwind(|| {
        // invoke closure
    });
    if result.is_ok() {
        // test failed — function did not raise
        mesh_test_fail_raise(file, file_len, line);
    }
    // else: panicked -> raised -> assert_raises passes
}
```

### Pattern 4: Mock Actor Cleanup Between Tests

The registry cleanup uses `mesh_actor_whereis` + `mesh_actor_exit` pattern. Before each test:

```rust
// mesh_test_cleanup_actors() -> void
// Kills all actors registered with names "test_mock_*" prefix or
// uses a thread-local list of Pids spawned since last cleanup.
#[no_mangle]
pub extern "C" fn mesh_test_cleanup_actors() {
    // Drain the thread-local mock actor list
    // Call mesh_actor_exit(pid) for each
}
```

The mock actor list is maintained in a thread_local! in test.rs (same pattern as LOCAL_REDUCTIONS in actor/mod.rs).

### Pattern 5: assert_receive

`assert_receive pattern, timeout_ms` lowers to:
1. Call `mesh_actor_receive(timeout_ms)` — returns `*const u8` (message ptr) or null on timeout
2. If null: call `mesh_test_fail_receive(pattern_src, timeout_ms, file, line)`
3. If non-null: apply the Mesh pattern match against the received message
4. If pattern match fails: call `mesh_test_fail_receive_mismatch(pattern_src, actual_str, file, line)`

The default 100ms timeout is supplied at the call-site during lowering (not a runtime default).

### Pattern 6: meshc test Subcommand

```rust
// Source: compiler/meshc/src/main.rs — follows existing Fmt/Build pattern
#[derive(Subcommand)]
enum Commands {
    // ... existing ...
    Test {
        /// Path to a specific test file (optional; runs all *.test.mpl if omitted)
        path: Option<PathBuf>,
        /// Show dots instead of names
        #[arg(long)]
        quiet: bool,
        /// Accept --coverage flag (stub)
        #[arg(long)]
        coverage: bool,
    },
}
```

The `test_runner.rs` structure:
```rust
pub fn run_tests(
    project_dir: &Path,
    filter_file: Option<&Path>,
    quiet: bool,
    coverage: bool,
) -> Result<TestSummary, String> {
    // 1. Discover *.test.mpl files (recursive, same pattern as collect_mesh_files)
    // 2. For each file: compile to temp binary via build(), execute, parse output
    // 3. Aggregate: total passed, failed, timing
    // 4. Print summary line
    // 5. Return exit code
}
```

### Anti-Patterns to Avoid

- **Adding new parser keywords**: `test`, `describe`, `setup`, `teardown` are NOT new keywords. They are registered as builtin functions. This avoids all lexer/parser changes.
- **Injecting test functions into normal builds**: The test DSL builtins are only available when the compiler is invoked via `meshc test` or processing `*.test.mpl` files. In `meshc build`, these names are unknown and cause a type error if used.
- **Blocking the scheduler in assert_receive**: Use the existing `mesh_actor_receive(timeout_ms)` spin-wait path (the main-thread spin-wait path at line 463 in actor/mod.rs), which works without being inside a coroutine.
- **New Cargo dependencies**: Everything needed is already in mesh-rt (actor, panic, GC, string).
- **Parsing test output from stdout**: Each test binary communicates results via exit code + structured output to stdout. The runner parses the structured stdout format (e.g., `PASS:test name\n` / `FAIL:test name:message\n` lines), NOT the human-readable colored output. The human-readable output is printed directly by the test binary to its own stdout, and the runner passes it through to the terminal.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Closure invocation in assert_raises | Custom unsafe fn pointer call wrapper | Existing mesh closure fn_ptr/env_ptr ABI from Http.stream (Phase 137) | Already handles the fn_ptr+env_ptr calling convention |
| Panic capture | New unwinding mechanism | `std::panic::catch_unwind` already used in actor crash isolation | Actor system already sets up the unwinding infrastructure |
| Temporary compilation | New build pipeline | Existing `build()` fn in meshc/src/main.rs | Full pipeline already works; test runner just calls it |
| File discovery | New walker | Existing `discover_mesh_files()` + extension filter change | Trivial extension check |
| ANSI colors | Color library dependency | Inline ANSI escape code constants | Only 3 colors needed; no dependency justified |
| Timing | External crate | `std::time::Instant` | Standard library suffices |

**Key insight:** The entire testing framework is built on top of the existing actor/panic infrastructure. No new runtime primitives are needed beyond thin wrappers that call existing functions.

---

## Common Pitfalls

### Pitfall 1: Test builtins visible in non-test builds
**What goes wrong:** User accidentally uses `assert_eq` in a regular `.mpl` file and gets confusing errors.
**Why it happens:** If test builtins are registered unconditionally in `builtins.rs`, they are always available.
**How to avoid:** Register test DSL builtins conditionally — only when the compiler is invoked in test mode. Pass a `is_test_file: bool` flag into `register_builtins()`, or register them separately in a `register_test_builtins()` called only for `.test.mpl` files.
**Warning signs:** e2e tests for non-test programs fail to detect `assert_eq` as an unknown name.

### Pitfall 2: Actor scheduler not running during tests
**What goes wrong:** Mock actors never receive messages because the M:N scheduler isn't started.
**Why it happens:** `mesh_rt_init_actor(N)` must be called before `mesh_rt_run_scheduler()`. The generated test main must call this.
**How to avoid:** The synthetic test main always calls `mesh_rt_init_actor(1)` at the top. Mock actors can run on 1 scheduler thread.
**Warning signs:** `assert_receive` always times out even with correct sends.

### Pitfall 3: assert_receive deadlock on main thread
**What goes wrong:** `assert_receive` hangs forever or crashes when called from the main thread.
**Why it happens:** The main thread is not a coroutine actor. Yield-based waiting doesn't work.
**How to avoid:** Use the spin-wait path in `mesh_actor_receive` — it already handles the non-coroutine case (see actor/mod.rs lines 463-485). Call `mesh_actor_receive(timeout_ms)` directly; the runtime selects the right path based on `CURRENT_YIELDER`.
**Warning signs:** Test hangs at assert_receive without ever timing out.

### Pitfall 4: Test binary output interleaving
**What goes wrong:** If the runner runs multiple test files concurrently and prints output inline, the colored output is interleaved and unreadable.
**Why it happens:** Parallel compilation + execution without output buffering.
**How to avoid:** In this phase, run test files **sequentially** (no parallelism flag). Each test binary's output is printed before the next starts. Parallelism is TEST-12 (future).
**Warning signs:** Colors and test names appear out of order.

### Pitfall 5: Structured output protocol collision
**What goes wrong:** Test assertion failure messages contain `PASS:` or `FAIL:` prefixes that confuse the runner's line parser.
**Why it happens:** If runner uses simple prefix parsing, assert messages containing those words break parsing.
**How to avoid:** Use a non-human-facing separator protocol, OR have the test binary exit with code 0/1 and only parse the exit code for pass/fail. The human-readable output is piped through unchanged. Keep the protocol simple: `exit(0)` = all tests passed, `exit(1)` = some test failed. The colored output is printed by the test binary directly; the runner just passes stdout/stderr through.
**Warning signs:** Runner incorrectly counts passes or failures.

### Pitfall 6: Actor registry leak between test files
**What goes wrong:** Named actors from one test file are still alive when the next test file's binary starts.
**Why it happens:** Named actors registered with `mesh_actor_register` persist in the global scheduler state.
**How to avoid:** Each test file compiles to a **separate binary** and runs in a **separate process**. Process exit kills all actors. No inter-file leak is possible with the separate-process model.
**Warning signs:** This is not actually a problem — separate processes provide perfect isolation.

### Pitfall 7: assert_eq type dispatch
**What goes wrong:** `assert_eq a, b` fails to compile when `a` and `b` are non-string types (Int, Bool, etc.) because the runtime function needs to print them.
**Why it happens:** The runtime needs to format both values as strings for the failure message.
**How to avoid:** The lowerer inserts a call to the appropriate `to_string` function based on the inferred type of `a`. For `Int`, call `mesh_int_to_string`. For `String`, use directly. For other types, use the existing `String.from()` polymorphic builtin. The actual runtime call is always `mesh_test_assert_eq(lhs_str, rhs_str, expr_src, file, line)` after the values are converted.
**Warning signs:** `assert_eq 1, 2` fails with a type error.

### Pitfall 8: Coverage flag confusion
**What goes wrong:** User expects `--coverage` to produce actual coverage data.
**Why it happens:** TEST-10 is explicitly stubbed per the STATE.md decision.
**How to avoid:** Print exactly "Coverage reporting coming soon" and exit 0 when `--coverage` is passed. Do not attempt any instrumentation.
**Warning signs:** Users file bug reports — this is acceptable, it's documented as a stub.

---

## Code Examples

### Test DSL Usage (the target surface area)

```
# Source: locked decisions in 138-CONTEXT.md

# foo.test.mpl — no imports needed

describe "User.greet" do
  setup do
    # runs before each test in this group
    let name = "Alice"
  end

  teardown do
    # runs after each test in this group
    # (cleanup)
  end

  test "returns hello" do
    assert_eq User.greet("Alice"), "Hello, Alice"
  end

  test "handles nil" do
    assert_raises fn() do
      User.greet(nil)
    end
  end
end

test "standalone test" do
  assert 1 + 1 == 2
  assert_ne "foo", "bar"
end

test "actor receives message" do
  let pid = Test.mock_actor(fn msg -> "ok" end)
  send(pid, "hello")
  assert_receive "hello", 500
end
```

### Runtime Function Signatures (mesh-rt/src/test.rs)

```rust
// Source: mesh-rt ABI contract, follows crypto.rs and actor/mod.rs patterns

/// mesh_test_begin(name_ptr: *const MeshString) -> void
/// Called before each test. In verbose mode, prints test name.
#[no_mangle]
pub extern "C" fn mesh_test_begin(name: *const MeshString) { ... }

/// mesh_test_pass() -> void
/// Called after a test body completes without panicking.
#[no_mangle]
pub extern "C" fn mesh_test_pass() { ... }

/// mesh_test_fail_msg(msg_ptr: *const MeshString) -> void
/// Called when an assertion fails. Prints message and records failure.
#[no_mangle]
pub extern "C" fn mesh_test_fail_msg(msg: *const MeshString) { ... }

/// mesh_test_assert(cond: i8, expr_src_ptr: *const MeshString,
///                  file_ptr: *const u8, file_len: u64, line: u32) -> void
#[no_mangle]
pub extern "C" fn mesh_test_assert(
    cond: i8,
    expr_src: *const MeshString,
    file: *const u8, file_len: u64, line: u32,
) { ... }

/// mesh_test_assert_eq(lhs: *const MeshString, rhs: *const MeshString,
///                     expr_src: *const MeshString,
///                     file: *const u8, file_len: u64, line: u32) -> void
/// Both lhs and rhs are already converted to strings by the lowerer.
#[no_mangle]
pub extern "C" fn mesh_test_assert_eq(
    lhs: *const MeshString, rhs: *const MeshString,
    expr_src: *const MeshString,
    file: *const u8, file_len: u64, line: u32,
) { ... }

/// mesh_test_assert_ne(lhs, rhs, expr_src, file, file_len, line) -> void
#[no_mangle]
pub extern "C" fn mesh_test_assert_ne(...) { ... }

/// mesh_test_assert_raises(fn_ptr: *const u8, env_ptr: *const u8,
///                          file: *const u8, file_len: u64, line: u32) -> void
/// Calls the closure; fails if it does not panic.
#[no_mangle]
pub extern "C" fn mesh_test_assert_raises(
    fn_ptr: *const u8, env_ptr: *const u8,
    file: *const u8, file_len: u64, line: u32,
) { ... }

/// mesh_test_mock_actor(fn_ptr: *const u8, env_ptr: *const u8) -> i64
/// Spawns a mock actor with the given message handler closure. Returns Pid.
#[no_mangle]
pub extern "C" fn mesh_test_mock_actor(
    fn_ptr: *const u8, env_ptr: *const u8,
) -> i64 { ... }

/// mesh_test_cleanup_actors() -> void
/// Kills and unregisters all mock actors created since last cleanup.
#[no_mangle]
pub extern "C" fn mesh_test_cleanup_actors() { ... }

/// mesh_test_summary(passed: i64, failed: i64, elapsed_ms: i64) -> void
/// Prints: "N passed, M failed in X.XXs"
#[no_mangle]
pub extern "C" fn mesh_test_summary(
    passed: i64, failed: i64, elapsed_ms: i64,
) { ... }
```

### Test Runner Discovery (meshc/src/test_runner.rs)

```rust
// Source: follows collect_mesh_files pattern in meshc/src/main.rs

pub fn discover_test_files(project_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    discover_test_recursive(project_root, project_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn discover_test_recursive(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        // Skip hidden directories
        if name.to_string_lossy().starts_with('.') { continue; }
        if path.is_dir() {
            discover_test_recursive(root, &path, files)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("mpl")
               && path.file_stem().and_then(|s| s.to_str())
                      .map(|s| s.ends_with(".test") || path.file_name()
                           .and_then(|n| n.to_str())
                           .map(|n| n.ends_with(".test.mpl"))
                           .unwrap_or(false))
                      .unwrap_or(false)
        {
            files.push(path.strip_prefix(root).unwrap().to_path_buf());
        }
    }
    Ok(())
}
```

### ANSI Color Constants

```rust
// Source: inline — no dependency needed
const GREEN: &str = "\x1b[32m";
const RED:   &str = "\x1b[31m";
const BOLD:  &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

// Pass: "  ✓ test name"  (green)
// Fail: "  ✗ test name"  (red, followed by failure detail)
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No testing framework | `meshc test` + `.test.mpl` DSL | Phase 138 | Mesh developers can write and run tests |
| Manual assertion via `if`/`panic` | `assert`, `assert_eq`, `assert_ne`, `assert_raises` | Phase 138 | Ergonomic testing surface |
| No mock actors | `Test.mock_actor()` + `assert_receive` | Phase 138 | Concurrency testing enabled |
| Coverage as future work | `--coverage` stub prints message | Phase 138 | Unblocks TEST-10; real impl is TEST-11 (v14.1) |

**Deprecated/outdated:**
- Manual `if cond then panic("...") end` pattern in tests: replaced by `assert`
- Running test logic in `main()` directly: replaced by `test "..." do ... end` blocks

---

## Open Questions

1. **How does the lowerer know it's processing a `.test.mpl` file?**
   - What we know: The file extension is the signal. The `build()` function in meshc receives the project dir and discovers files.
   - What's unclear: Does the file extension need to flow into the MIR lowerer, or is the transformation done at the CLI level by generating a wrapper source?
   - Recommendation: Thread `is_test_mode: bool` through `lower_to_mir_raw()`. When true, enable test DSL builtin registration and the harness generation. Alternative: generate a wrapper `.mpl` string that calls an internal test runner. The former is cleaner.

2. **How does `assert` capture the expression source text?**
   - What we know: The source text must be extracted at compile time (not runtime), since the compiled binary doesn't have the source. The Rust test infrastructure uses a proc macro for this.
   - What's unclear: Does the Mesh lowerer have access to the original source span to extract the expression text?
   - Recommendation: Yes — the MIR lowerer has access to `TextRange` (via rowan) and the source string. At lowering time, extract the span text and embed it as a string literal in the MIR. This is the same mechanism used for `file` and `line` in `mesh_panic`.

3. **How does `assert_receive` pattern matching work at the MIR level?**
   - What we know: Mesh pattern matching is fully implemented in the lowerer for `match` expressions.
   - What's unclear: Can `assert_receive msg_pattern, 100` be lowered as syntactic sugar for `receive do msg_pattern -> true; _ -> false end` with a timeout?
   - Recommendation: Yes. Lower `assert_receive pattern, timeout` to: `let __msg = mesh_actor_receive(timeout); if __msg == null then fail_timeout else match __msg with pattern -> () | _ -> fail_mismatch end`. This reuses all existing pattern matching infrastructure.

4. **What is the exact protocol between test binary and runner?**
   - What we know: Simpler is better. Exit code 0 = all pass, exit code 1 = any fail.
   - What's unclear: Does the runner need to know per-test results to format the summary, or does the test binary print everything and the runner just passes it through?
   - Recommendation: The test binary prints all human-readable output (colored pass/fail lines, failure details, summary) directly to stdout. The runner just passes stdout through to the terminal and collects the exit code. The runner's summary is per-file: "file.test.mpl: 3 passed / file2.test.mpl: 1 failed". This keeps the protocol maximally simple.

---

## Sources

### Primary (HIGH confidence)
- `compiler/meshc/src/main.rs` — Existing subcommand structure; `build()` pipeline and `collect_mesh_files()` pattern verified directly
- `compiler/mesh-codegen/src/mir/lower.rs` (lines 682-945, 10862-10980) — 5-point registration pattern verified directly; `STDLIB_MODULES`, `map_builtin_name`, `known_functions`
- `compiler/mesh-typeck/src/infer.rs` (lines 212-501, 1639-1661) — `stdlib_modules()` HashMap and `STDLIB_MODULE_NAMES` verified directly
- `compiler/mesh-codegen/src/codegen/intrinsics.rs` (lines 283-369) — LLVM declaration pattern verified directly
- `compiler/mesh-rt/src/actor/mod.rs` (lines 131-150, 437-485, 755-822) — `mesh_actor_spawn`, `mesh_actor_receive`, `mesh_actor_register`; spin-wait main-thread path verified
- `compiler/mesh-rt/src/panic.rs` — `mesh_panic` ABI and `catch_unwind` usage pattern
- `compiler/mesh-rt/src/io.rs` — `MeshResult`/`alloc_result` pattern
- `compiler/mesh-rt/src/crypto.rs` — `extern "C"` function pattern for new runtime module
- `.planning/STATE.md` (line 59) — "each *.test.mpl is a complete Mesh program; runner compiles and executes each independently"
- `compiler/mesh-common/src/token.rs` — All 49 keywords; confirmed `test`, `describe`, `setup`, `teardown` are NOT reserved keywords (safe to use as function names)

### Secondary (MEDIUM confidence)
- Pattern analysis of Phase 137 Http.stream implementation (fn_ptr/env_ptr ABI for closures) — directly applicable to `assert_raises` and `Test.mock_actor`

### Tertiary (LOW confidence)
- None — all findings verified from primary sources

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components verified in existing codebase
- Architecture: HIGH — 5-point registration pattern is established and proven; test binary approach confirmed in STATE.md
- Pitfalls: HIGH — identified from direct inspection of existing implementation constraints
- Open questions: MEDIUM — recommendations are well-grounded but involve design choices

**Research date:** 2026-02-28
**Valid until:** 2026-03-30 (codebase is stable; patterns well-established)
