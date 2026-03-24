---
estimated_steps: 4
estimated_files: 5
skills_used:
  - debug-like-expert
---

# T02: Inline supported control flow in user and stream services

**Slice:** S03 — Request, handler, and control-flow dogfood cleanup
**Milestone:** M032

## Description

Retire the two stale single-use wrapper helpers in `mesher/services/user.mpl` and `mesher/services/stream_manager.mpl` by using the already-supported inline handler control flow directly in the real service bodies. The main risk here is over-cleanup: `stream_manager.mpl` still carries the real nested-`&&` keep-site, so this task must narrow its edits to the stale helper/comment pair only.

## Steps

1. Inspect the single-use helper sites in `mesher/services/user.mpl` and `mesher/services/stream_manager.mpl` alongside the named proofs in `compiler/meshc/tests/e2e.rs`, and treat `both_match(...)` in `stream_manager.mpl` as an explicit keep-site guard.
2. Inline the `case authenticate_user(...)` flow directly inside `UserService.Login`, remove `login_user(...)` if it becomes unused, and preserve the current `Err(_) -> Err("authentication failed")` behavior.
3. Inline the `if is_stream_client(...)` branch directly inside `StreamManager.BufferMessage`, remove `buffer_if_client(...)` if it becomes unused, and leave `both_match(...)` and its comment untouched.
4. Run the supported-path tests, the nested-`&&` keep-site guard, the stale/retained comment greps, and the mesher format/build checks so the slice closes on real dogfood proof.

## Must-Haves

- [ ] `mesher/services/user.mpl` no longer needs `login_user(...)`; `UserService.Login` uses the supported inline `case` pattern and preserves behavior.
- [ ] `mesher/services/stream_manager.mpl` no longer needs `buffer_if_client(...)`; `StreamManager.BufferMessage` uses the supported inline `if/else` pattern and preserves behavior.
- [ ] The real nested-`&&` keep-site in `mesher/services/stream_manager.mpl` and the timer keep-sites in `mesher/services/writer.mpl` / `mesher/ingestion/pipeline.mpl` remain intact.

## Verification

- `cargo test -q -p meshc --test e2e e2e_m032_supported_service_call_case -- --nocapture`
- `cargo test -q -p meshc --test e2e e2e_m032_supported_cast_if_else -- --nocapture`
- `cargo test -q -p meshc --test e2e e2e_m032_limit_nested_and -- --nocapture`
- `! rg -n "complex case expressions|parser limitation with if/else in cast handlers" mesher/services/user.mpl mesher/services/stream_manager.mpl && rg -n "avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl`
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- build mesher`

## Observability Impact

- Signals added/changed: none; handler-control-flow regressions remain visible through the existing supported-path tests and the nested-`&&` keep-site guard.
- How a future agent inspects this: rerun the three verification tests above, inspect the exact comment greps, and use `meshc` format/build output to confirm mesher still composes cleanly.
- Failure state exposed: a bad inline rewrite fails the supported handler tests or mesher build, while accidental keep-site damage shows up in the retained-comment grep.

## Inputs

- `mesher/services/user.mpl` — stale service-call helper/comment pair targeted for cleanup.
- `mesher/services/stream_manager.mpl` — stale cast-helper/comment pair plus the real nested-`&&` keep-site that must remain.
- `compiler/meshc/tests/e2e.rs` — supported service-call/cast proofs and nested-`&&` keep-site guard.
- `mesher/services/writer.mpl` — retained timer keep-site wording that must survive this slice unchanged.
- `mesher/ingestion/pipeline.mpl` — retained timer keep-site wording that must survive this slice unchanged.

## Expected Output

- `mesher/services/user.mpl` — inline login handler flow replaces the stale helper/comment path.
- `mesher/services/stream_manager.mpl` — inline buffer-message handler flow replaces the stale helper/comment path without touching the real keep-site.
