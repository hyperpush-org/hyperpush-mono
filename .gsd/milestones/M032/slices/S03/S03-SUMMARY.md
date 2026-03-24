---
id: S03
parent: M032
milestone: M032
provides:
  - Mesher now dogfoods direct request-query handling and inline handler control flow in the audited request/user/stream modules, with the stale folklore comments removed and the real keep-sites left explicit.
requires:
  - slice: S01
    provides: stale-vs-real comment classification plus named supported-path and retained-limit proofs for the audited modules.
affects:
  - S05
key_files:
  - mesher/ingestion/routes.mpl
  - mesher/services/user.mpl
  - mesher/services/stream_manager.mpl
  - compiler/meshc/tests/e2e.rs
  - compiler/meshc/tests/e2e_stdlib.rs
  - .gsd/milestones/M032/slices/S03/S03-UAT.md
key_decisions:
  - D050: retire stale request/handler folklore by dogfooding supported inline Mesh patterns directly in mesher while preserving only the verified route-closure, nested-&&, and timer keep-sites.
patterns_established:
  - Prefer direct handler-level dogfooding over wrapper helpers when the point of the slice is to prove Mesh already supports the underlying pattern.
  - Keep stale-comment removal paired with explicit retained-limit guards so cleanup does not silently erase still-real compiler/runtime limits.
observability_surfaces:
  - cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture
  - cargo test -q -p meshc --test e2e e2e_m032_supported_service_call_case -- --nocapture
  - cargo test -q -p meshc --test e2e e2e_m032_supported_cast_if_else -- --nocapture
  - cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture
  - cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture
  - rg -n "HTTP routing does not support closures|avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/ingestion/routes.mpl mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl
  - cargo run -q -p meshc -- fmt --check mesher
  - cargo run -q -p meshc -- build mesher
drill_down_paths:
  - .gsd/milestones/M032/slices/S03/tasks/T01-SUMMARY.md
  - .gsd/milestones/M032/slices/S03/tasks/T02-SUMMARY.md
duration: two task handoffs plus closer verification replay
verification_result: passed
completed_at: 2026-03-24
---

# S03: Request, handler, and control-flow dogfood cleanup

**Mesher now uses `Request.query(...)`, inline service-call `case`, and inline cast-handler `if/else` directly in the audited modules, while the real route-closure, nested-`&&`, and timer keep-sites remain explicit and green.**

## What Happened

S03 closed the stale-folklore cleanup that S01 scoped but did not execute. In `mesher/ingestion/routes.mpl`, `handle_list_issues(...)` no longer hardcodes `"unresolved"` behind a stale “query parsing is not available” comment. It now reads `Request.query(request, "status")` directly and defaults through a local `case` to `"unresolved"`, which is the exact supported path the slice set out to dogfood.

In `mesher/services/user.mpl`, the single-use `login_user(...)` wrapper and its stale service-call limitation comment are gone. `UserService.Login` now returns `(pool, case authenticate_user(...) do ...)` directly, while preserving the existing `Err(_) -> Err("authentication failed")` behavior.

In `mesher/services/stream_manager.mpl`, the single-use `buffer_if_client(...)` wrapper and its stale cast-handler limitation comment are gone. `StreamManager.BufferMessage` now uses the inline `if is_stream_client(...) do ... else ... end` shape directly inside the cast body.

The slice stayed disciplined about what not to “clean up.” The real keep-sites remained intact: the bare-function HTTP route note in `mesher/ingestion/routes.mpl`, the `both_match(...)` helper and nested-`&&` comment in `mesher/services/stream_manager.mpl`, and the `Timer.send_after` comments in `mesher/services/writer.mpl` and `mesher/ingestion/pipeline.mpl`. That keeps the code truthful instead of swapping one kind of folklore for another.

## Verification

I reran the full slice gate from the plan against the current tree, not the stale task artifact:

- `cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture`
- `cargo test -q -p meshc --test e2e e2e_m032_supported_service_call_case -- --nocapture`
- `cargo test -q -p meshc --test e2e e2e_m032_supported_cast_if_else -- --nocapture`
- `cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture`
- `cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture`
- `! rg -n "query string parsing not available in Mesh|complex case expressions|parser limitation with if/else in cast handlers" mesher/ingestion/routes.mpl mesher/services/user.mpl mesher/services/stream_manager.mpl`
- `rg -n "HTTP routing does not support closures|avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/ingestion/routes.mpl mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl`
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- build mesher`

All checks passed. The verification surface also stayed informative: the supported-path tests isolate each retired workaround family, while the nested-`&&` and route-closure tests plus the keep-site grep confirm the remaining comments still point at live limitations rather than drifted folklore.

## Requirements Advanced

- R011 — This slice kept the work anchored to real mesher handlers and services instead of introducing synthetic examples or speculative language work.
- R035 — The audited request/handler/control-flow workaround comments in mesher now better match current verified reality, while the still-real keep-sites remain named and guarded.

## Requirements Validated

- none

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

The closer had to rebuild the slice closeout from task evidence and a fresh verification replay because the stored `T02-VERIFY.json` artifact had a truncated keep-site grep command for the `avoids && ...` comment, and the expected slice summary/UAT had not been written yet.

## Known Limitations

- S03 does not retire the real retained limits: bare HTTP route closures still fail at live request time, nested `&&` still needs `both_match(...)`, and `Timer.send_after` still does not deliver a service-dispatchable cast payload.
- S04 still owns the mixed-truth module-boundary cleanup.
- S05 still owns the integrated Mesher proof and the final retained-limit ledger.

## Follow-ups

- S04 should simplify only the module-boundary workaround family and leave the S03 keep-sites alone unless new proof lands first.
- S05 should carry this slice’s stale-comment removals and retained-comment grep into the final ledger so the milestone closeout reflects current truth instead of the old folklore set.
- Future closers should trust the live slice gate over stale task verification JSON if the stored command strings are obviously truncated or malformed.

## Files Created/Modified

- `mesher/ingestion/routes.mpl` — replaced the hardcoded issue-status path with direct `Request.query(...)` defaulting and removed the stale query-parsing comment.
- `mesher/services/user.mpl` — removed the stale login wrapper/comment pair and inlined the supported service-call `case` directly in `UserService.Login`.
- `mesher/services/stream_manager.mpl` — removed the stale buffer wrapper/comment pair and inlined the supported cast-handler `if/else` directly in `StreamManager.BufferMessage`.
- `.gsd/milestones/M032/slices/S03/S03-UAT.md` — records the artifact-driven acceptance script for this cleanup slice.
- `.gsd/REQUIREMENTS.md` — updates R011 and R035 metadata to reflect S03’s actual coverage.
- `.gsd/DECISIONS.md` — records the direct dogfood cleanup choice as D050.
- `.gsd/KNOWLEDGE.md` — records the supported direct handler shapes and the nearby real keep-sites so future cleanup work does not over-correct them.

## Forward Intelligence

### What the next slice should know
- The supported direct patterns are now established in real mesher code: local `Request.query(...)` defaulting in handlers, inline `case` in service `call` bodies, and inline `if/else` in service `cast` bodies.
- The surrounding keep-sites are still real. Do not remove them unless a new proof replaces them.

### What's fragile
- Comment-truth cleanup in `mesher/` is fragile where stale and real limitation notes sit close together. A mechanically broad “remove workaround comments” pass can easily erase the actual retained-limit markers that S05 still needs.

### Authoritative diagnostics
- `cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture`, `...e2e_m032_supported_service_call_case...`, and `...e2e_m032_supported_cast_if_else...` — fastest truthful proof that the retired workaround families are genuinely supported.
- `cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture` plus `cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture` — authoritative guardrails for the still-real keep-sites.

### What assumptions changed
- The original assumption was that these request/handler wrapper helpers were still needed to keep mesher building.
- What actually happened was narrower and better: Mesh already supported the underlying handler shapes, so the honest fix was to dogfood them directly and leave only the genuinely still-broken patterns commented.
