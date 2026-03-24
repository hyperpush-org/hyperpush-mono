# S03: Request, handler, and control-flow dogfood cleanup

**Goal:** Retire the stale request/handler/control-flow workaround comments in the audited `mesher/` modules by using the already-supported Mesh patterns directly, while preserving the nearby real keep-sites and keeping mesher's build/format proof truthful.
**Demo:** In the real `mesher/` codebase, `handle_list_issues(...)` reads the optional `status` query directly with a default, `UserService.Login` and `StreamManager.BufferMessage` inline their supported control flow, the three stale folklore comments/helpers are gone, and the named guard tests plus mesher `fmt`/`build` checks still pass.

## Must-Haves

- `mesher/ingestion/routes.mpl` dogfoods `Request.query(...)` directly in `handle_list_issues(...)`, removes the stale query-parsing comment, and keeps the real bare-route closure keep-site comment intact.
- `mesher/services/user.mpl` and `mesher/services/stream_manager.mpl` inline the already-supported handler control flow, remove the stale single-use wrapper comments/helpers, and preserve current behavior.
- Slice verification stays anchored to real `mesher/` files and real proof surfaces: the supported-path tests, the route-closure and nested-`&&` keep-site guards, the stale/retained comment greps, and `cargo run -q -p meshc -- fmt --check mesher` plus `cargo run -q -p meshc -- build mesher` all pass.

## Proof Level

- This slice proves: contract and integration
- Real runtime required: yes
- Human/UAT required: no

## Verification

- `compiler/meshc/tests/e2e.rs` via `cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture`
- `compiler/meshc/tests/e2e.rs` via `cargo test -q -p meshc --test e2e e2e_m032_supported_service_call_case -- --nocapture`
- `compiler/meshc/tests/e2e.rs` via `cargo test -q -p meshc --test e2e e2e_m032_supported_cast_if_else -- --nocapture`
- `compiler/meshc/tests/e2e.rs` via `cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture`
- `compiler/meshc/tests/e2e_stdlib.rs` via `cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture`
- `rg -n "query string parsing not available in Mesh|complex case expressions|parser limitation with if/else in cast handlers" mesher/ingestion/routes.mpl mesher/services/user.mpl mesher/services/stream_manager.mpl` returns no matches, and `rg -n "HTTP routing does not support closures|avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/ingestion/routes.mpl mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl` returns the retained keep-sites
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- build mesher`

## Observability / Diagnostics

- Runtime signals: named M032 e2e failures localize request-query support, service-call control flow, cast-handler control flow, nested-`&&` keep-site drift, and route-closure runtime drift without adding new instrumentation.
- Inspection surfaces: `compiler/meshc/tests/e2e.rs`, `compiler/meshc/tests/e2e_stdlib.rs`, grep over the audited `mesher/` comments, and `cargo run -q -p meshc -- build mesher`.
- Failure visibility: compiler/test output identifies the broken proof surface, while stale-comment grep hits or missing keep-site grep hits identify comment-truth drift directly in `mesher/`.
- Redaction constraints: none.

## Integration Closure

- Upstream surfaces consumed: `Request.query(...)`, `UserService.Login`, `StreamManager.BufferMessage`, and the existing M032 proof tests in `compiler/meshc/tests/e2e.rs` and `compiler/meshc/tests/e2e_stdlib.rs`.
- New wiring introduced in this slice: direct request-query usage in `mesher/ingestion/routes.mpl` and direct inline control flow in two mesher service handlers; no new entrypoints or runtime components.
- What remains before the milestone is truly usable end-to-end: S04 must clean the mixed-truth module-boundary comments, and S05 must run the integrated closeout proof and publish the retained-limit ledger.

## Tasks

- [x] **T01: Dogfood direct request-query handling in ingestion routes** `est:45m`
  - Why: This is the cleanest stale-folklore site and directly advances R035/R011 by replacing a disproven request-parsing workaround in a real mesher HTTP handler.
  - Files: `mesher/ingestion/routes.mpl`, `mesher/api/search.mpl`, `mesher/api/helpers.mpl`, `compiler/meshc/tests/e2e.rs`, `compiler/meshc/tests/e2e_stdlib.rs`
  - Do: Replace the hardcoded `"unresolved"` path in `handle_list_issues(...)` with direct inline `Request.query(request, "status")` defaulting to `"unresolved"`; remove only the stale query-parsing comment; preserve the top-of-file bare-function route keep-site and existing handler response semantics.
  - Verify: `cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture && cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture && ! rg -n "query string parsing not available in Mesh" mesher/ingestion/routes.mpl && rg -n "HTTP routing does not support closures" mesher/ingestion/routes.mpl && cargo run -q -p meshc -- build mesher`
  - Done when: `handle_list_issues(...)` reads the optional `status` query with a `"unresolved"` fallback, the stale comment is gone, the route-closure keep-site remains, and mesher still builds.
- [ ] **T02: Inline supported control flow in user and stream services** `est:1h`
  - Why: This retires the remaining stale handler/control-flow folklore in real mesher services without disturbing the verified nested-`&&` keep-site that still belongs to S05.
  - Files: `mesher/services/user.mpl`, `mesher/services/stream_manager.mpl`, `compiler/meshc/tests/e2e.rs`, `mesher/services/writer.mpl`, `mesher/ingestion/pipeline.mpl`
  - Do: Inline the `authenticate_user(...)` case directly inside `UserService.Login`, delete `login_user(...)` if it becomes unused, inline the `is_stream_client(...)` branch directly inside `StreamManager.BufferMessage`, delete `buffer_if_client(...)` if it becomes unused, and preserve `both_match(...)` plus the timer keep-sites untouched.
  - Verify: `cargo test -q -p meshc --test e2e e2e_m032_supported_service_call_case -- --nocapture && cargo test -q -p meshc --test e2e e2e_m032_supported_cast_if_else -- --nocapture && cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture && ! rg -n "complex case expressions|parser limitation with if/else in cast handlers" mesher/services/user.mpl mesher/services/stream_manager.mpl && rg -n "avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl && cargo run -q -p meshc -- fmt --check mesher && cargo run -q -p meshc -- build mesher`
  - Done when: the two stale helper/comment sites are removed, `both_match(...)` and the timer keep-sites still exist, the focused handler tests pass, and mesher format/build proof stays green.

## Files Likely Touched

- `mesher/ingestion/routes.mpl`
- `mesher/services/user.mpl`
- `mesher/services/stream_manager.mpl`
