---
phase: 138-testing-framework
plan: 05
subsystem: testing
tags: [testing-framework, assert_receive, preprocessor, typeck, mesh-rt]

# Dependency graph
requires:
  - phase: 138-testing-framework
    provides: "Plans 01-04: meshc test runner, assertion DSL, describe/setup/teardown, Test.mock_actor"
provides:
  - "assert_receive PATTERN, TIMEOUT DSL keyword preprocessed to receive blocks in test bodies"
  - "test_fail_msg registered as typeck builtin for test bodies"
  - "lib.rs pub use re-exports for mesh_test_run_body, mesh_test_mock_actor, mesh_test_pass_count, mesh_test_fail_count"
  - "ACTOR_MSG_TYPE_KEY injected for __test_body_ functions enabling receive/self() in test bodies"
  - "test_mock_actor.test.mpl fixture with passing assert_receive tests"
affects: [future-testing, mesh-actor, assert_receive-users]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "assert_receive preprocessor: text-level transformation before typeck runs"
    - "Test body actor context injection: __test_body_ fns get ACTOR_MSG_TYPE_KEY so receive blocks are permitted"
    - "Single-line receive block generation: avoids parser newline-before-end issue in parse_receive_expr"

key-files:
  created: []
  modified:
    - compiler/meshc/src/test_runner.rs
    - compiler/mesh-typeck/src/builtins.rs
    - compiler/mesh-typeck/src/infer.rs
    - compiler/mesh-rt/src/lib.rs
    - tests/e2e/test_mock_actor.test.mpl

key-decisions:
  - "LOCKED: assert_receive failure message format: 'assert_receive {pattern} timed out after {timeout_ms}ms'"
  - "Single-line receive block: receive do PATTERN -> () after TIMEOUT -> test_fail_msg(...) end on one line to avoid parser issue where parse_receive_expr does not eat newlines before END_KW after after clause"
  - "ACTOR_MSG_TYPE_KEY injection: __test_body_ functions get actor message type context because the main thread has a process entry (PID) created by mesh_rt_init_actor, making receive semantically valid at runtime"
  - "test_fail_msg registered as String->Unit builtin — it was used in generated code but not registered in builtins.rs"
  - "Use self() (function call form) not bare self in test bodies — bare self is a NAME_REF for impl method receivers, not actor self"
  - "Default timeout for assert_receive with no explicit timeout: 100ms"

patterns-established:
  - "Assert receive pattern: send before assert_receive when testing self-receive in test body"
  - "Preprocessor transform: split_assert_receive_args handles comma inside brackets for complex patterns"

requirements-completed: [TEST-09]

# Metrics
duration: 21min
completed: 2026-02-28
---

# Phase 138 Plan 05: assert_receive Preprocessor and Gap Closure Summary

**assert_receive DSL keyword implemented via source-level preprocessor: transforms test body lines into single-line receive do...after...end blocks, with ACTOR_MSG_TYPE_KEY injection enabling receive in test body functions**

## Performance

- **Duration:** 21 min
- **Started:** 2026-02-28T23:16:35Z
- **Completed:** 2026-02-28T23:37:35Z
- **Tasks:** 2 (Task 3 was verification only, no new commits)
- **Files modified:** 5

## Accomplishments
- assert_receive PATTERN, TIMEOUT is now a valid DSL keyword in *.test.mpl files; preprocessor transforms it to single-line receive blocks with timeout failure messages
- test_mock_actor.test.mpl updated with 3 passing tests including two assert_receive tests using self() for actor PID self-send
- Closed lib.rs gap: mesh_test_run_body, mesh_test_mock_actor, mesh_test_pass_count, mesh_test_fail_count now in pub use re-exports
- Closed typeck gap: test_fail_msg registered as String->Unit builtin; assert_receive registered defensively as Int->Unit
- ACTOR_MSG_TYPE_KEY injected for __test_body_ functions enabling receive/self() in compiler-generated test body fns

## Task Commits

Each task was committed atomically:

1. **Task 1: assert_receive preprocessor and builtin registration** - `9a899059` (feat)
2. **Task 2: complete implementation and fixture** - `97118f85` (feat)

## Files Created/Modified
- `compiler/meshc/src/test_runner.rs` - Added transform_assert_receive(), split_assert_receive_args(); apply transform to setup/body/teardown; generate single-line receive blocks
- `compiler/mesh-typeck/src/builtins.rs` - Registered test_fail_msg (String->Unit) and assert_receive (Int->Unit) as builtins
- `compiler/mesh-typeck/src/infer.rs` - Injected ACTOR_MSG_TYPE_KEY into infer_fn_def for __test_body_ functions
- `compiler/mesh-rt/src/lib.rs` - Added mesh_test_run_body, mesh_test_mock_actor, mesh_test_pass_count, mesh_test_fail_count to pub use
- `tests/e2e/test_mock_actor.test.mpl` - Updated with assert_receive tests using self() form; all 3 tests pass

## Decisions Made

1. **Single-line receive block**: The Mesh parser's `parse_receive_expr` does not call `eat_newlines()` before checking for `END_KW` after the after clause. Generated multi-line receive blocks with newlines fail to parse. The fix was generating `receive do PATTERN -> () after TIMEOUT -> test_fail_msg("...") end` on a single line.

2. **ACTOR_MSG_TYPE_KEY injection**: The test body functions (`__test_body_N`) are regular `fn` definitions but run in the main thread which has an actor process entry (PID created by `mesh_rt_init_actor`). Injecting ACTOR_MSG_TYPE_KEY lets the type checker allow `receive` and `self()` expressions in these functions. The runtime supports this because `mesh_actor_receive` handles the non-coroutine (main thread) case via spin-wait.

3. **self() not bare self**: Bare `self` in Mesh is parsed as `NAME_REF` (for impl method receivers), NOT as `SelfExpr`. The `SelfExpr` is only created for `self()` (with parentheses). Test fixtures must use `self()` to get the actor's own PID.

4. **test_fail_msg was not a registered builtin**: The function was referenced in generated code (via map_builtin_name in lower.rs) but not registered in builtins.rs, causing type errors when assert_receive was used. Added as String->Unit builtin.

5. **Default timeout 100ms**: When assert_receive is used without a timeout argument (e.g., `assert_receive 42`), the default timeout is 100ms.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Parser does not eat newlines before END_KW after receive after clause**
- **Found during:** Task 2 (fixture testing)
- **Issue:** `parse_receive_expr` in the Mesh parser does not call `eat_newlines()` before checking for `END_KW` after the after clause body. Multi-line receive blocks with the `end` on a separate line fail to parse with "expected `end` to close `receive` block".
- **Fix:** Changed `transform_assert_receive` to generate single-line receive blocks: `receive do PATTERN -> () after TIMEOUT -> test_fail_msg("...") end` on one line.
- **Files modified:** `compiler/meshc/src/test_runner.rs`
- **Verification:** `meshc test test_mock_actor.test.mpl` compiles and runs
- **Committed in:** 97118f85 (Task 2 commit)

**2. [Rule 1 - Bug] test_fail_msg not registered in builtins.rs**
- **Found during:** Task 2 (fixture testing)
- **Issue:** `test_fail_msg` is called in the generated receive after-clause body but was not registered as a builtin in `builtins.rs`. The typechecker reported "undefined variable: test_fail_msg".
- **Fix:** Added `test_fail_msg` as String->Unit builtin to `builtins.rs`.
- **Files modified:** `compiler/mesh-typeck/src/builtins.rs`
- **Verification:** Type error gone; test compiles
- **Committed in:** 97118f85 (Task 2 commit)

**3. [Rule 1 - Bug] receive/self() unavailable in test body functions**
- **Found during:** Task 2 (fixture testing)
- **Issue:** Test body functions (`fn __test_body_N()`) are plain `fn` definitions. The typechecker's `infer_receive_expr` and `infer_self_expr` require `ACTOR_MSG_TYPE_KEY` to be in the environment. Without it, E0017 "receive used outside actor block" is raised. Plan assumed test bodies had actor context.
- **Fix:** In `infer_fn_def`, inject `ACTOR_MSG_TYPE_KEY` with a fresh type variable when the function name starts with `__test_body_`. This is semantically correct because the main thread has an actor process entry (PID) created by `mesh_rt_init_actor`.
- **Files modified:** `compiler/mesh-typeck/src/infer.rs`
- **Verification:** E0017 no longer raised for test body fns; receive blocks work at runtime
- **Committed in:** 97118f85 (Task 2 commit)

**4. [Rule 1 - Bug] Bare self not usable in test bodies (use self() instead)**
- **Found during:** Task 2 (fixture testing)
- **Issue:** Bare `self` in Mesh is parsed as `NAME_REF` (for impl method receivers), not as `SelfExpr`. The plan and initial fixture used `let me = self` (bare self). This causes "undefined variable: self" because "self" as a string is not in scope.
- **Fix:** Changed fixture to use `let me = self()` (function call form) which parses as `SelfExpr` and uses `ACTOR_MSG_TYPE_KEY` correctly.
- **Files modified:** `tests/e2e/test_mock_actor.test.mpl`
- **Verification:** Tests compile and pass; self() returns the main thread's PID
- **Committed in:** 97118f85 (Task 2 commit)

---

**Total deviations:** 4 auto-fixed (all Rule 1 - Bug)
**Impact on plan:** All fixes essential for correctness. No scope creep. The plan's design intent (assert_receive via receive blocks) was sound; the implementation needed 4 bug fixes to match the runtime's capabilities.

## Issues Encountered

The `parse_receive_expr` bug (newlines before END_KW after after clause) is a pre-existing parser issue not introduced by this plan. It only manifests when `receive do...after...end` blocks are used in contexts where multi-line formatting is typical. The single-line workaround avoids the parser limitation without modifying the parser (which would be a larger change).

## Next Phase Readiness

- assert_receive is now fully functional: preprocessor transforms DSL, typeck allows receive in test bodies, runtime receives messages via main thread mailbox
- All 6 test fixtures pass (24 tests total); full workspace builds clean
- TEST-09 requirement satisfied
- Phase 138 testing framework is now complete (all gaps from verification report closed: TEST-07 setup/teardown in Plan 04, TEST-09 assert_receive in this plan, lib.rs re-exports fixed)

---
*Phase: 138-testing-framework*
*Completed: 2026-02-28*

## Self-Check: PASSED

All 7 expected files exist. Both task commits verified (9a899059, 97118f85). All key content present in modified files.
