---
id: T03
parent: S01
milestone: M029
provides:
  - Canonical dotted imports across `reference-backend/` plus a clean formatter round-trip proof
key_files:
  - reference-backend/main.mpl
  - reference-backend/api/health.mpl
  - reference-backend/storage/jobs.mpl
  - reference-backend/api/router.mpl
  - reference-backend/api/jobs.mpl
  - reference-backend/jobs/worker.mpl
  - .gsd/milestones/M029/slices/S01/tasks/T03-PLAN.md
  - .gsd/milestones/M029/slices/S01/S01-PLAN.md
key_decisions: []
patterns_established:
  - Use one real `meshc fmt` rewrite plus exact-output tests and the `^from .*\. ` grep sweep to prove dogfood sources are clean, because `fmt --check` alone cannot detect already-stabilized dotted-path corruption.
observability_surfaces:
  - cargo run -q -p meshc -- fmt --check reference-backend
  - "! rg -n \"^from .*\\. \" reference-backend -g '*.mpl'"
  - cargo test -q -p meshc --test e2e_fmt fmt_preserves_dotted_paths_exactly -- --nocapture
  - cargo test -q -p mesh-fmt --lib walk_path_preserves_dotted_import_and_impl_paths -- --nocapture
duration: 10m
verification_result: passed
completed_at: 2026-03-24 11:41:57 EDT
blocker_discovered: false
---

# T03: Repair reference-backend imports and prove round-trip cleanliness

**Repaired `reference-backend` dotted imports and proved the backend round-trips cleanly under `meshc fmt`.**

## What Happened

I repaired the remaining stale dotted imports in `reference-backend/main.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/router.mpl`, `reference-backend/api/jobs.mpl`, and `reference-backend/jobs/worker.mpl`, restoring them to canonical `Foo.Bar` form. `reference-backend/api/health.mpl` was already repaired in T01, so this task preserved its parenthesized multiline import shape instead of rewriting it again.

Per the pre-flight contract, I also added `## Observability Impact` to `.gsd/milestones/M029/slices/S01/tasks/T03-PLAN.md` before implementation. After the source repairs, I ran `cargo run -q -p meshc -- fmt reference-backend`, let the formatter normalize the backend once, and then used `meshc fmt --check` plus the repo-wide stale-import grep as the acceptance gate.

The formatter round-trip is now clean: `reference-backend/` no longer contains `Api. Router` / `Foo. Bar` style import corruption, and the multiline parenthesized import in `api/health.mpl` remains intact after formatting.

## Verification

I ran the real formatter over `reference-backend/`, then reran `meshc fmt --check` to confirm the rewritten backend is stable on a second pass. I also reran the full slice verification set so the task closes with the same exact-output library and CLI regressions that caught the original formatter bug.

`reference-backend/api/health.mpl` was inspected after formatting and still begins with a parenthesized multiline `from Jobs.Worker import (` block, which is the smoke target for this slice’s multiline-import contract.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -q -p mesh-fmt --lib` | 0 | ✅ pass | 0.57s |
| 2 | `cargo test -q -p meshc --test e2e_fmt -- --nocapture` | 0 | ✅ pass | 6.00s |
| 3 | `cargo test -q -p meshc --test e2e_fmt fmt_preserves_dotted_paths_exactly -- --nocapture` | 0 | ✅ pass | 6.34s |
| 4 | `cargo test -q -p meshc --test e2e e2e_multiline_import_paren -- --nocapture` | 0 | ✅ pass | 8.80s |
| 5 | `cargo run -q -p meshc -- fmt --check reference-backend` | 0 | ✅ pass | 6.48s |
| 6 | `! rg -n "^from .*\. " reference-backend -g '*.mpl'` | 0 | ✅ pass | 0.07s |
| 7 | `cargo test -q -p mesh-fmt --lib walk_path_preserves_dotted_import_and_impl_paths -- --nocapture` | 0 | ✅ pass | 0.43s |

## Diagnostics

Use `cargo run -q -p meshc -- fmt --check reference-backend` as the directory-level round-trip signal; if this regresses, stderr will show the formatter diff for whichever backend file drifted. Use `! rg -n "^from .*\. " reference-backend -g '*.mpl'` as the cheapest corruption sweep for stale spaced imports. For exact textual regression checks, use `cargo test -q -p meshc --test e2e_fmt fmt_preserves_dotted_paths_exactly -- --nocapture` at the CLI layer and `cargo test -q -p mesh-fmt --lib walk_path_preserves_dotted_import_and_impl_paths -- --nocapture` at the walker layer.

## Deviations

- I updated `.gsd/milestones/M029/slices/S01/tasks/T03-PLAN.md` first to add the missing `## Observability Impact` section required by the pre-flight checks.
- `reference-backend/api/health.mpl` did not need another source edit in T03 because T01 had already repaired it to unblock the formatter library fixture; this task verified that formatting preserved its multiline parenthesized import shape.

## Known Issues

None.

## Files Created/Modified

- `reference-backend/main.mpl` — restored canonical dotted imports and let `meshc fmt` normalize the file.
- `reference-backend/storage/jobs.mpl` — restored the canonical `Types.Job` import and normalized the file with `meshc fmt`.
- `reference-backend/api/router.mpl` — restored canonical dotted imports for `Api.Health` and `Api.Jobs`.
- `reference-backend/api/jobs.mpl` — restored canonical dotted imports for `Types.Job`, `Storage.Jobs`, and `Runtime.Registry`.
- `reference-backend/jobs/worker.mpl` — restored canonical dotted imports for `Types.Job` and `Storage.Jobs`.
- `.gsd/milestones/M029/slices/S01/tasks/T03-PLAN.md` — added the required observability-impact section before execution.
- `.gsd/milestones/M029/slices/S01/S01-PLAN.md` — marked T03 complete.
