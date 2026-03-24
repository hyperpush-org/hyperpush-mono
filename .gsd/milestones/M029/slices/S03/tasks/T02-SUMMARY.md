---
id: T02
parent: S03
milestone: M029
provides:
  - Mesher API and service modules now use the canonical parenthesized multiline import shape, and the repo-wide Mesher long-import grep is green
key_files:
  - mesher/api/alerts.mpl
  - mesher/api/dashboard.mpl
  - mesher/api/team.mpl
  - mesher/services/project.mpl
  - mesher/services/user.mpl
  - .gsd/milestones/M029/slices/S03/S03-PLAN.md
  - .gsd/milestones/M029/slices/S03/tasks/T02-PLAN.md
key_decisions:
  - Reused `reference-backend/api/health.mpl` verbatim as the multiline import shape for all five `Storage.Queries` rewrites and kept the work limited to human-authored import cleanup rather than mixing in formatter-wave churn
patterns_established:
  - Once T01 and T02 are complete, any remaining S03 failures are formatter/UAT backlog rather than leftover over-120-character Mesher import lines; confirm that state with the repo-wide long-import grep before touching compiler or source-shape logic again
observability_surfaces:
  - "! rg -n '^from .{121,}' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl"
  - "! rg -n '^from .*\\. ' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl"
  - "! rg -n '^from .{121,}' mesher -g '*.mpl'"
  - "cargo run -q -p meshc -- fmt --check mesher"
  - "cargo run -q -p meshc -- fmt --check reference-backend"
  - "cargo run -q -p meshc -- build mesher"
  - "cargo run -q -p meshc -- build reference-backend"
  - "! rg -n '^from .*\\. ' mesher reference-backend -g '*.mpl'"
  - "(cargo run -q -p meshc -- fmt --check mesher > /tmp/m029-s03-fmt-mesher.log 2>&1 && test ! -s /tmp/m029-s03-fmt-mesher.log) || (rg -n 'error|panic|from .*\\. ' /tmp/m029-s03-fmt-mesher.log && false)"
  - "test -f .gsd/milestones/M029/slices/S03/S03-UAT.md"
duration: 10m
verification_result: passed
completed_at: 2026-03-24T12:50:15-04:00
blocker_discovered: false
---

# T02: Rewrite API and service imports to canonical multiline form

**Rewrote Mesher’s remaining API/service `Storage.Queries` imports to the canonical multiline form and cleared the repo-wide long-import gate.**

## What Happened

I fixed the pre-flight artifact gap first. `.gsd/milestones/M029/slices/S03/tasks/T02-PLAN.md` now includes the required `## Observability Impact` section before the source edits.

The source change stayed narrow. I reused `reference-backend/api/health.mpl` as the exact import-shape anchor and rewrote the long `from Storage.Queries import ...` lines in `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/team.mpl`, `mesher/services/project.mpl`, and `mesher/services/user.mpl` into the same parenthesized multiline form with one imported name per line and the closing `)` on its own line. Imported names and ordering were left unchanged.

I did not touch compiler code or `reference-backend/` source. After these five rewrites, the repo-wide Mesher long-import grep is now green, so the remaining red slice work is the planned formatter/UAT backlog rather than any leftover over-120-character import lines.

No new test file was added. This task is source-only import normalization, so the truthful verification surface is the targeted grep pair plus the slice-level formatter/build checks.

## Verification

Task-level verification passed:
- `! rg -n '^from .{121,}' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl`
- `! rg -n '^from .*\. ' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl`

Slice-level verification is partial, as expected on T02:
- The repo-wide long-import grep now passes across all of `mesher/`.
- `cargo run -q -p meshc -- fmt --check mesher` is still red and reports the expected 35-file formatter backlog owned by T03-T06, including the five files touched here.
- `cargo run -q -p meshc -- fmt --check reference-backend`, both dogfood build commands, and the repo-wide spaced-dotted-path grep all pass.
- The captured formatter-log diagnostic check is still red because `/tmp/m029-s03-fmt-mesher.log` is non-empty with `would reformat` entries; it does not show `error`, `panic`, or spaced dotted import corruption.
- `test -f .gsd/milestones/M029/slices/S03/S03-UAT.md` still fails because T06 has not written the final UAT artifact yet.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `! rg -n '^from .{121,}' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl` | 0 | ✅ pass | 0.07s |
| 2 | `! rg -n '^from .*\. ' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl` | 0 | ✅ pass | 0.05s |
| 3 | `! rg -n '^from .{121,}' mesher -g '*.mpl'` | 0 | ✅ pass | 0.13s |
| 4 | `cargo run -q -p meshc -- fmt --check mesher` | 1 | ❌ fail | 6.8s |
| 5 | `cargo run -q -p meshc -- fmt --check reference-backend` | 0 | ✅ pass | 5.94s |
| 6 | `cargo run -q -p meshc -- build mesher` | 0 | ✅ pass | 10.81s |
| 7 | `cargo run -q -p meshc -- build reference-backend` | 0 | ✅ pass | 8.03s |
| 8 | `! rg -n '^from .*\. ' mesher reference-backend -g '*.mpl'` | 0 | ✅ pass | 0.1s |
| 9 | `(cargo run -q -p meshc -- fmt --check mesher > /tmp/m029-s03-fmt-mesher.log 2>&1 && test ! -s /tmp/m029-s03-fmt-mesher.log) \|\| (rg -n 'error\|panic\|from .*\. ' /tmp/m029-s03-fmt-mesher.log && false)` | 1 | ❌ fail | 6.24s |
| 10 | `test -f .gsd/milestones/M029/slices/S03/S03-UAT.md` | 1 | ❌ fail | 0.01s |

## Diagnostics

No runtime signals changed. The durable inspection surfaces for this task are the two targeted negative greps on the five touched files, the repo-wide long-import grep, `cargo run -q -p meshc -- fmt --check mesher`, `cargo run -q -p meshc -- fmt --check reference-backend`, both dogfood build commands, the repo-wide spaced-dotted-path grep, and `/tmp/m029-s03-fmt-mesher.log` for captured Mesher formatter diagnostics.

If later work regresses this task, start with the two task-local greps and the repo-wide long-import grep. If those stay green but slice verification still fails, inspect `/tmp/m029-s03-fmt-mesher.log`: at this point it should only show the 35-file formatter backlog, not parser errors or `Storage. Queries`-style corruption.

## Deviations

- Updated `.gsd/milestones/M029/slices/S03/tasks/T02-PLAN.md` before implementation to add the missing `## Observability Impact` section required by the pre-flight check.

## Known Issues

- `cargo run -q -p meshc -- fmt --check mesher` still reports 35 files that would be reformatted; the broader canonicalization wave is still owned by T03-T06.
- The five files touched here still appear in that formatter backlog because this task intentionally stopped at manual import-shape cleanup rather than running the formatter early.
- `.gsd/milestones/M029/slices/S03/S03-UAT.md` does not exist yet; T06 owns the final UAT artifact.

## Files Created/Modified

- `mesher/api/alerts.mpl` — rewrote the long `Storage.Queries` import into the canonical parenthesized multiline form.
- `mesher/api/dashboard.mpl` — rewrote the long `Storage.Queries` import into the canonical parenthesized multiline form.
- `mesher/api/team.mpl` — rewrote the long `Storage.Queries` import into the canonical parenthesized multiline form.
- `mesher/services/project.mpl` — rewrote the long `Storage.Queries` import into the canonical parenthesized multiline form.
- `mesher/services/user.mpl` — rewrote the long `Storage.Queries` import into the canonical parenthesized multiline form.
- `.gsd/milestones/M029/slices/S03/tasks/T02-PLAN.md` — added the required observability-impact section for this task.
- `.gsd/milestones/M029/slices/S03/S03-PLAN.md` — marked T02 complete in the slice task list.
