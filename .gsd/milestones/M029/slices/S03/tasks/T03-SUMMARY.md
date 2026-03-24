---
id: T03
parent: S03
milestone: M029
provides:
  - Mesher entrypoint and API modules are now on canonical formatter output, reducing the Mesher formatter backlog from 35 files to 27 while keeping the rewritten multiline imports intact
key_files:
  - mesher/main.mpl
  - mesher/api/alerts.mpl
  - mesher/api/dashboard.mpl
  - mesher/api/detail.mpl
  - mesher/api/helpers.mpl
  - mesher/api/search.mpl
  - mesher/api/settings.mpl
  - mesher/api/team.mpl
  - .gsd/milestones/M029/slices/S03/tasks/T03-PLAN.md
  - .gsd/milestones/M029/slices/S03/S03-PLAN.md
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Accepted the formatter-authored canonical output for `mesher/main.mpl` and `mesher/api/*.mpl` once scoped `fmt --check`, dotted-path greps, and both dogfood builds stayed green, instead of hand-restoring the previous spacing style
  - Kept the task inside Mesher source cleanup; no compiler work was started because this formatter wave did not reproduce multiline-import collapse or dotted-path corruption
patterns_established:
  - After T03, a red `cargo run -q -p meshc -- fmt --check mesher` should list only downstream ingestion/storage/service/types/tests/migrations files; if `mesher/main.mpl` or `mesher/api/*.mpl` reappear there, treat that as a real regression in this wave
  - The fixed formatter's current canonical output still inserts blank lines between adjacent comment lines and can staircase long pipe chains; for this slice that is acceptable canonical churn as long as scoped `fmt --check` passes, multiline imports stay multiline, dotted-path greps stay clean, and both builds remain green
observability_surfaces:
  - "cargo run -q -p meshc -- fmt mesher/main.mpl && cargo run -q -p meshc -- fmt mesher/api && cargo run -q -p meshc -- fmt --check mesher/main.mpl && cargo run -q -p meshc -- fmt --check mesher/api"
  - "! rg -n '^from .{121,}' mesher/main.mpl mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl && ! rg -n '^from .*\\. ' mesher/main.mpl mesher/api -g '*.mpl'"
  - "cargo run -q -p meshc -- fmt --check mesher"
  - "cargo run -q -p meshc -- fmt --check reference-backend"
  - "cargo run -q -p meshc -- build mesher"
  - "cargo run -q -p meshc -- build reference-backend"
  - "! rg -n '^from .*\\. ' mesher reference-backend -g '*.mpl'"
  - "/tmp/m029-s03-fmt-mesher.log"
duration: 7m
verification_result: passed
completed_at: 2026-03-24T12:57:24-04:00
blocker_discovered: false
---

# T03: Canonicalize Mesher entrypoint and API modules with the fixed formatter

**Moved Mesher’s entrypoint and API modules onto the formatter’s canonical output without regressing multiline imports or dotted paths.**

## What Happened

I fixed the pre-flight artifact gap first. `.gsd/milestones/M029/slices/S03/tasks/T03-PLAN.md` now has the required `## Observability Impact` section before the source work.

Then I read the full eight-file wave (`mesher/main.mpl` plus all seven `mesher/api/*.mpl` files), snapshotted their pre-format contents, and ran the scoped formatter commands exactly as planned: `cargo run -q -p meshc -- fmt mesher/main.mpl` followed by `cargo run -q -p meshc -- fmt mesher/api`.

I inspected the post-format diffs with special attention to `mesher/main.mpl`, `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, and `mesher/api/team.mpl`. The formatter preserved the parenthesized multiline imports and did not introduce any spaced dotted module paths. It did apply broader mechanical churn inside this wave: blank lines between adjacent comment lines, additional spacing inside some pattern matches and `json { ... }` literals, and very deep staircase indentation for the long router pipeline in `mesher/main.mpl`. Those changes are ugly but they are formatter-authored canonical output in the current toolchain, not a fresh import-shape regression.

Because the scoped formatter round-trip and the follow-up build/grep checks all stayed green, I did not start any compiler work. I also recorded the formatter-output gotcha in `.gsd/KNOWLEDGE.md` so later S03 tasks do not mistake that canonical churn for a new failure mode.

No new test file was added. This task is formatter/build compliance work, so the truthful verification surface is the scoped formatter command, the import-shape greps, the slice-level formatter/build gates, and the captured Mesher formatter log.

## Verification

Task-level verification passed:
- `cargo run -q -p meshc -- fmt mesher/main.mpl && cargo run -q -p meshc -- fmt mesher/api && cargo run -q -p meshc -- fmt --check mesher/main.mpl && cargo run -q -p meshc -- fmt --check mesher/api`
- `! rg -n '^from .{121,}' mesher/main.mpl mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl && ! rg -n '^from .*\. ' mesher/main.mpl mesher/api -g '*.mpl'`

Slice-level verification is partial, as expected on T03:
- The repo-wide Mesher long-import grep stays green.
- `cargo run -q -p meshc -- fmt --check mesher` is still red, but the remaining backlog has dropped from 35 files to 27 and now consists only of downstream ingestion, storage, service, migration, test, and type files owned by T04-T06.
- `cargo run -q -p meshc -- fmt --check reference-backend`, `cargo run -q -p meshc -- build mesher`, `cargo run -q -p meshc -- build reference-backend`, and the repo-wide spaced-dotted-path grep all pass.
- The captured formatter-log diagnostic check stays red because `/tmp/m029-s03-fmt-mesher.log` still contains the expected `would reformat` backlog for untouched Mesher files.
- `test -f .gsd/milestones/M029/slices/S03/S03-UAT.md` still fails because T06 owns the final UAT artifact.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo run -q -p meshc -- fmt mesher/main.mpl && cargo run -q -p meshc -- fmt mesher/api && cargo run -q -p meshc -- fmt --check mesher/main.mpl && cargo run -q -p meshc -- fmt --check mesher/api` | 0 | ✅ pass | 25.37s |
| 2 | `! rg -n '^from .{121,}' mesher/main.mpl mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl && ! rg -n '^from .*\. ' mesher/main.mpl mesher/api -g '*.mpl'` | 0 | ✅ pass | 0.14s |
| 3 | `! rg -n '^from .{121,}' mesher -g '*.mpl'` | 0 | ✅ pass | 0.06s |
| 4 | `cargo run -q -p meshc -- fmt --check mesher` | 1 | ❌ fail | 6.27s |
| 5 | `cargo run -q -p meshc -- fmt --check reference-backend` | 0 | ✅ pass | 5.94s |
| 6 | `cargo run -q -p meshc -- build mesher` | 0 | ✅ pass | 11.56s |
| 7 | `cargo run -q -p meshc -- build reference-backend` | 0 | ✅ pass | 7.91s |
| 8 | `! rg -n '^from .*\. ' mesher reference-backend -g '*.mpl'` | 0 | ✅ pass | 0.03s |
| 9 | `(cargo run -q -p meshc -- fmt --check mesher > /tmp/m029-s03-fmt-mesher.log 2>&1 && test ! -s /tmp/m029-s03-fmt-mesher.log) \|\| (rg -n 'error\|panic\|from .*\. ' /tmp/m029-s03-fmt-mesher.log && false)` | 1 | ❌ fail | 6.01s |
| 10 | `test -f .gsd/milestones/M029/slices/S03/S03-UAT.md` | 1 | ❌ fail | 0.01s |

## Diagnostics

There are no runtime signals for this task. The durable inspection surfaces are the scoped formatter round-trip command for `mesher/main.mpl` and `mesher/api`, the two import-shape greps, `cargo run -q -p meshc -- fmt --check mesher`, `cargo run -q -p meshc -- fmt --check reference-backend`, both build commands, the repo-wide spaced-dotted-path grep, and `/tmp/m029-s03-fmt-mesher.log`.

If this task appears to regress later, start with the task-level formatter round-trip and the targeted grep pair. If those stay green, inspect `/tmp/m029-s03-fmt-mesher.log` and confirm whether the red surface is still the expected downstream 27-file backlog rather than `mesher/main.mpl` or `mesher/api/*.mpl` re-entering the formatter list.

## Deviations

- Updated `.gsd/milestones/M029/slices/S03/tasks/T03-PLAN.md` before implementation to add the missing `## Observability Impact` section required by the pre-flight check.
- Updated `.gsd/KNOWLEDGE.md` to record that the current fixed formatter preserves multiline imports and dotted paths in this wave but still emits aggressive comment-spacing and pipeline-indentation churn.

## Known Issues

- `cargo run -q -p meshc -- fmt --check mesher` is still red for 27 untouched files in `mesher/ingestion`, `mesher/storage`, `mesher/services`, `mesher/migrations`, `mesher/tests`, and `mesher/types`; that backlog is owned by T04-T06.
- `.gsd/milestones/M029/slices/S03/S03-UAT.md` does not exist yet; T06 owns the final UAT artifact.

## Files Created/Modified

- `mesher/main.mpl` — moved the entrypoint file onto canonical formatter output while preserving the multiline imports.
- `mesher/api/alerts.mpl` — moved the alert handlers module onto canonical formatter output.
- `mesher/api/dashboard.mpl` — moved the dashboard handlers module onto canonical formatter output.
- `mesher/api/detail.mpl` — moved the event detail handlers module onto canonical formatter output.
- `mesher/api/helpers.mpl` — moved the shared API helpers module onto canonical formatter output.
- `mesher/api/search.mpl` — moved the search handlers module onto canonical formatter output.
- `mesher/api/settings.mpl` — moved the settings/storage handlers module onto canonical formatter output.
- `mesher/api/team.mpl` — moved the team/API-key handlers module onto canonical formatter output while preserving the multiline import.
- `.gsd/milestones/M029/slices/S03/tasks/T03-PLAN.md` — added the required observability-impact section for this task.
- `.gsd/milestones/M029/slices/S03/S03-PLAN.md` — marked T03 complete in the slice task list.
- `.gsd/KNOWLEDGE.md` — recorded the current formatter-output behavior for later S03 waves.
