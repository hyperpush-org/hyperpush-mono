# S03: Request, handler, and control-flow dogfood cleanup — UAT

**Milestone:** M032
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: this slice is a truthful mesher cleanup pass, so the authoritative acceptance path is the named compiler tests, comment-truth greps, and mesher fmt/build gates against the real codebase.

## Preconditions

- Run from the repo root: `/Users/sn0w/Documents/dev/mesh-lang`
- Rust/Cargo and the repo’s normal native build prerequisites are available
- No unrelated local edits are pending in `mesher/` or `compiler/meshc/tests/`
- No temporary acceptance artifacts from earlier runs are masking file contents

## Smoke Test

Run:

`cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture`

**Expected:** Cargo reports `running 1 test`, the test passes, and the result is `1 passed; 0 failed`.

## Test Cases

### 1. Mesher issue listing now dogfoods direct `Request.query(...)`

1. Run `rg -n "Request\.query\(request, \"status\"\)|None -> \"unresolved\"" mesher/ingestion/routes.mpl`
2. Confirm the output points at `handle_list_issues(...)`.
3. Run `cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture`
4. **Expected:** the source shows direct query lookup plus a local `None -> "unresolved"` fallback, and the named test passes.

### 2. `UserService.Login` now uses inline service-call `case` handling

1. Run `rg -n "call Login|authenticate_user\(|login_user\(" mesher/services/user.mpl`
2. Confirm `call Login` still exists, `authenticate_user(...)` is used inside it, and there is no remaining `fn login_user(...)` helper.
3. Run `cargo test -q -p meshc --test e2e e2e_m032_supported_service_call_case -- --nocapture`
4. **Expected:** `UserService.Login` is the direct dogfood site for inline `case` handling, the wrapper helper is gone, and the named test passes.

### 3. `StreamManager.BufferMessage` now uses inline cast-handler `if/else`

1. Run `rg -n "cast BufferMessage|if is_stream_client\(|buffer_if_client\(" mesher/services/stream_manager.mpl`
2. Confirm `cast BufferMessage` contains the inline `if is_stream_client(...)` branch and there is no remaining `fn buffer_if_client(...)` helper.
3. Run `cargo test -q -p meshc --test e2e e2e_m032_supported_cast_if_else -- --nocapture`
4. **Expected:** `BufferMessage` is now the direct dogfood site for inline cast-handler control flow, the wrapper helper is gone, and the named test passes.

### 4. Stale workaround comments are gone but the real keep-sites remain

1. Run `! rg -n "query string parsing not available in Mesh|complex case expressions|parser limitation with if/else in cast handlers" mesher/ingestion/routes.mpl mesher/services/user.mpl mesher/services/stream_manager.mpl`
2. Run `rg -n "HTTP routing does not support closures|avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/ingestion/routes.mpl mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl`
3. Run `cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture`
4. Run `cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture`
5. **Expected:** the stale folklore strings are absent, the four retained keep-site comments are still present, and both retained-limit tests pass.

### 5. Mesher still formats and builds cleanly after the cleanup

1. Run `cargo run -q -p meshc -- fmt --check mesher`
2. Run `cargo run -q -p meshc -- build mesher`
3. **Expected:** `fmt --check` exits 0 silently, and `build mesher` exits 0 with `Compiled: mesher/mesher` in the output.

## Edge Cases

### Missing `status` query still defaults to `unresolved`

1. Run `rg -n "None -> \"unresolved\"" mesher/ingestion/routes.mpl`
2. **Expected:** the explicit default branch is present in `handle_list_issues(...)`; the slice did not replace the old hardcoded default with a required query parameter.

### Real keep-sites were not “cleaned up” by accident

1. Run `rg -n "both_match\(|HTTP routing does not support closures|Timer.send_after delivers raw bytes" mesher/services/stream_manager.mpl mesher/ingestion/routes.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl`
2. **Expected:** all of those retained-limit markers are still present in their real files. If they disappear while the supported-path tests still pass, the slice has regressed from truthful cleanup into over-cleanup.

## Failure Signals

- `handle_list_issues(...)` still hardcodes `"unresolved"` without reading `Request.query(...)`
- `mesher/services/user.mpl` still contains `login_user(...)` or the stale “complex case expressions” comment
- `mesher/services/stream_manager.mpl` still contains `buffer_if_client(...)` or the stale cast-handler limitation comment
- The retained keep-site grep loses the route-closure, nested-`&&`, or timer comments
- `e2e_m032_limit_nested_and` or `e2e_m032_route_closure_runtime_failure` stops passing, which would mean the slice erased or invalidated a real retained-limit proof
- `cargo run -q -p meshc -- fmt --check mesher` or `cargo run -q -p meshc -- build mesher` fails after the cleanup

## Requirements Proved By This UAT

- R011 — the cleanup is proven on real mesher request/service paths rather than on synthetic examples alone
- R035 — the audited request/handler/control-flow workaround comments now reflect current verified reality in the touched mesher modules

## Not Proven By This UAT

- The S04 module-boundary workaround convergence work
- The S05 integrated retained-limit ledger and milestone-wide closeout proof
- Any claim that the retained route-closure, nested-`&&`, or timer limits are fixed; this UAT proves they still remain real keep-sites

## Notes for Tester

- This slice is intentionally narrow. If a check fails, start by comparing the failing file against the stale-comment grep and the retained keep-site grep before changing compiler code.
- The named tests are the fastest truthful signals for this slice. Do not replace them with broader suite runs unless a narrower check points at a deeper regression.
