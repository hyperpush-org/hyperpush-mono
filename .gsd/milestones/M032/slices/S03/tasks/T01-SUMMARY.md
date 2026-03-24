---
id: T01
parent: S03
milestone: M032
provides:
  - Direct request-query handling in mesher ingestion issue listing with a local unresolved fallback
key_files:
  - mesher/ingestion/routes.mpl
  - .gsd/milestones/M032/slices/S03/S03-PLAN.md
  - .gsd/milestones/M032/slices/S03/tasks/T01-SUMMARY.md
key_decisions:
  - Kept the Request.query defaulting logic inline in handle_list_issues so the audited stale-folklore site now proves direct handler support without adding another helper
patterns_established:
  - Mesher HTTP handlers can default Request.query(request, ...) locally through Option case matching while preserving existing response behavior
observability_surfaces:
  - compiler/meshc/tests/e2e.rs::e2e_m032_supported_request_query
  - compiler/meshc/tests/e2e_stdlib.rs::e2e_m032_route_closure_runtime_failure
  - rg checks over mesher/ingestion/routes.mpl and the slice-wide stale/keep-site comments
  - cargo run -q -p meshc -- build mesher
duration: 28m
verification_result: passed
completed_at: 2026-03-24T18:32:01-04:00
blocker_discovered: false
---

# T01: Dogfood direct request-query handling in ingestion routes

**Dogfooded Request.query in mesher issue listing with an unresolved fallback.**

## What Happened

I replaced the hardcoded `"unresolved"` status path in `mesher/ingestion/routes.mpl` with a direct `Request.query(request, "status")` read that falls back to `"unresolved"` when the query is absent. I removed only the stale query-parsing folklore comment and left the real top-of-file route-closure keep-site untouched.

I also ran the task contract checks plus the full slice verification set to establish the post-T01 baseline. All named tests, `mesher` format, and `mesher` build passed. The only slice-level miss is the expected stale-comment grep in `mesher/services/user.mpl` and `mesher/services/stream_manager.mpl`, which remains for T02.

## Verification

Verified the direct request-query support path with the named M032 e2e guard, verified the retained bare-function route limitation with the stdlib runtime guard, confirmed the target stale comment is gone while the real keep-site remains, and rebuilt `mesher` through `meshc`. I also ran the remaining slice checks to capture the current baseline before T02.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture` | 0 | ✅ pass | 8.75s |
| 2 | `cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture` | 0 | ✅ pass | 7.82s |
| 3 | `! rg -n "query string parsing not available in Mesh" mesher/ingestion/routes.mpl && rg -n "HTTP routing does not support closures" mesher/ingestion/routes.mpl` | 0 | ✅ pass | 0.13s |
| 4 | `cargo run -q -p meshc -- build mesher` | 0 | ✅ pass | 13.55s |
| 5 | `cargo test -q -p meshc --test e2e e2e_m032_supported_service_call_case -- --nocapture` | 0 | ✅ pass | 8.69s |
| 6 | `cargo test -q -p meshc --test e2e e2e_m032_supported_cast_if_else -- --nocapture` | 0 | ✅ pass | 8.28s |
| 7 | `cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture` | 0 | ✅ pass | 6.79s |
| 8 | `rg -n "query string parsing not available in Mesh|complex case expressions|parser limitation with if/else in cast handlers" mesher/ingestion/routes.mpl mesher/services/user.mpl mesher/services/stream_manager.mpl && rg -n "HTTP routing does not support closures|avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/ingestion/routes.mpl mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl` | 1 | ❌ fail | 0.09s |
| 9 | `cargo run -q -p meshc -- fmt --check mesher` | 0 | ✅ pass | 6.53s |

## Diagnostics

Future inspection stays on the existing M032 surfaces: rerun `e2e_m032_supported_request_query` for direct request-query support, rerun `e2e_m032_route_closure_runtime_failure` for the retained route-closure limitation, grep `mesher/ingestion/routes.mpl` for the removed stale comment and preserved keep-site, and rebuild with `cargo run -q -p meshc -- build mesher`.

## Deviations

None.

## Known Issues

- The slice-wide stale-comment grep still fails because `mesher/services/user.mpl` and `mesher/services/stream_manager.mpl` retain the expected T02 comments.

## Files Created/Modified

- `mesher/ingestion/routes.mpl` — replaced the hardcoded issue-status argument with direct `Request.query(...)` defaulting and removed the stale folklore comment.
- `.gsd/milestones/M032/slices/S03/tasks/T01-SUMMARY.md` — recorded implementation details and verification evidence for this task.
- `.gsd/milestones/M032/slices/S03/S03-PLAN.md` — marked T01 complete.
