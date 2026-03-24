---
estimated_steps: 4
estimated_files: 5
skills_used:
  - debug-like-expert
---

# T01: Dogfood direct request-query handling in ingestion routes

**Slice:** S03 — Request, handler, and control-flow dogfood cleanup
**Milestone:** M032

## Description

Replace the stale request-query workaround in `mesher/ingestion/routes.mpl` with the already-supported direct `Request.query(...)` pattern, while preserving the nearby real keep-site that route handlers must stay bare functions. This task closes the simplest audited stale-folklore site first and gives the slice an early real-mesher proof point.

## Steps

1. Inspect `handle_list_issues(...)` in `mesher/ingestion/routes.mpl` against the existing direct `Request.query(...)` examples in `mesher/api/search.mpl` and `mesher/api/helpers.mpl`, and keep the top-of-file route-closure warning in scope as a do-not-touch guard.
2. Replace the hardcoded `"unresolved"` argument with an inline `Request.query(request, "status")` default to `"unresolved"`, remove only the stale query-parsing comment, and preserve the handler's current response/error behavior.
3. Run the supported request-query proof plus the route-closure keep-site proof, then grep the target file to confirm the stale comment is gone and the keep-site comment remains.
4. Rebuild `mesher` through `meshc` so this file's real dogfood path still compiles in the product entrypoint.

## Must-Haves

- [ ] `mesher/ingestion/routes.mpl` uses direct `Request.query(...)` in `handle_list_issues(...)` with a `"unresolved"` fallback.
- [ ] The stale query-parsing comment is removed without changing the real route-closure keep-site at the top of the file.
- [ ] The task proves the cleanup against the named compiler/runtime guardrails instead of relying on source inspection alone.

## Verification

- `cargo test -q -p meshc --test e2e e2e_m032_supported_request_query -- --nocapture`
- `cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture`
- `! rg -n "query string parsing not available in Mesh" mesher/ingestion/routes.mpl && rg -n "HTTP routing does not support closures" mesher/ingestion/routes.mpl`
- `cargo run -q -p meshc -- build mesher`

## Observability Impact

- Signals added/changed: none; support-vs-keep-site drift remains visible through the existing named tests and exact comment greps.
- How a future agent inspects this: rerun the two verification tests above, inspect `mesher/ingestion/routes.mpl`, and rebuild via `cargo run -q -p meshc -- build mesher`.
- Failure state exposed: request-query regressions fail the supported-path test, while accidental keep-site removal or stale folklore retention shows up immediately in the grep checks.

## Inputs

- `mesher/ingestion/routes.mpl` — target handler plus adjacent real route-closure keep-site.
- `mesher/api/search.mpl` — existing direct `Request.query(...)` pattern already used in mesher handlers.
- `mesher/api/helpers.mpl` — existing `Request.query(...) -> Option<String>` defaulting contract.
- `compiler/meshc/tests/e2e.rs` — supported request-query proof used as the task contract.
- `compiler/meshc/tests/e2e_stdlib.rs` — runtime route-closure keep-site proof that must remain true.

## Expected Output

- `mesher/ingestion/routes.mpl` — direct request-query dogfood replaces the stale workaround wording and hardcoded status path.
