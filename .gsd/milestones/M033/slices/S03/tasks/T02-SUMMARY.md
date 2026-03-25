---
id: T02
parent: S03
milestone: M033
key_files:
  - compiler/meshc/tests/e2e_m033_s03.rs
  - mesher/storage/queries.mpl
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Changed `check_new_issue` and `should_fire_by_cooldown` from direct `Repo.exists(...)` returns to `Repo.all(...) ?` plus a `List.length(rows) > 0` check because the copied Mesher probe compile path breaks on the direct `Bool ! String` `Repo.exists` form.
  - Treated the failing `composed_reads` storage probes as a proof-surface blocker rather than changing caller contracts or faking passing coverage; the remaining T02 proofs need a higher-level Mesher verification surface or a compiler/runtime fix.
duration: ""
verification_result: mixed
completed_at: 2026-03-25T19:30:06.228Z
blocker_discovered: true
---

# T02: Attempted the T02 composed-read proof expansion, fixed the probe-compatible boolean helpers, and recorded a storage-probe blocker for the remaining read families.

**Attempted the T02 composed-read proof expansion, fixed the probe-compatible boolean helpers, and recorded a storage-probe blocker for the remaining read families.**

## What Happened

I started by repairing the carry-forward T01 harness breakage so the S03 test target was usable again: removed the bad escaped quotes from the Rust-authored Mesh probe templates, added the missing `Types.User.Session` import in the session probe, and changed `mesher/storage/queries.mpl` so `check_new_issue` and `should_fire_by_cooldown` use the probe-safe `Repo.all(...) ?` + `List.length(rows) > 0` pattern instead of direct `Repo.exists(...)`. After that, `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture` passed and `cargo run -q -p meshc -- build mesher` passed, which confirmed the S03 harness and the current Mesher query file were back in a healthy baseline state.

I then extended `compiler/meshc/tests/e2e_m033_s03.rs` with T02-specific seed helpers and initial `e2e_m033_s03_composed_reads_*` probes for the joined, aggregate, detail/list, and alert/cooldown families. The read-side Mesher functions in `mesher/storage/queries.mpl` were already largely rewritten onto the builder surface before this recovery attempt, so the remaining work in this unit was proof construction rather than another round of raw-SQL retirement.

That proof expansion exposed a real blocker in the current storage-only probe surface. The copied Mesher probe can successfully exercise map-returning basic reads, but the T02 families hit deeper compiler/runtime limits: imported struct-list results such as `list_issues_by_status(...)` trigger LLVM verifier/runtime failures when the probe tries to consume them, and some aggregate/map-returning paths stringify selected values as raw pointer addresses instead of real strings once the probe stages them through helper bindings. I narrowed those failures with targeted reruns (`composed_reads`, the joined/team filter, and the dashboard aggregate filter), and recorded the recovery facts in `.gsd/KNOWLEDGE.md`. At this point the remaining proof work needs a different verification surface than the storage-only probe pattern the task plan assumed — most likely a higher-level Mesher/app harness or a compiler/runtime fix first — so I am marking this task as blocked for replan instead of pretending the T02 proof bundle is done.

## Verification

Verified that the repaired baseline still works by rerunning `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture` and `cargo run -q -p meshc -- build mesher`, both of which passed after the `Repo.exists` rewrite and session-probe import fix. Then ran the T02 target `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture` plus narrower `joined_issue_and_team_rows` and `dashboard_aggregates` filters. Those probes failed for concrete compiler/runtime reasons: the joined/team proof hit imported struct-list/LLVM verifier failures, and the aggregate probe surfaced pointer-stringification instead of real row values. I did not proceed to the broader slice-level fmt/check/verify-script commands after that because the storage-probe blocker invalidated the current T02 proof path.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture` | 0 | ✅ pass | 67340ms |
| 2 | `cargo run -q -p meshc -- build mesher` | 0 | ✅ pass | 17000ms |
| 3 | `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture` | 101 | ❌ fail | 116210ms |
| 4 | `cargo test -p meshc --test e2e_m033_s03 e2e_m033_s03_composed_reads_joined_issue_and_team_rows -- --nocapture` | 101 | ❌ fail | 36380ms |
| 5 | `cargo test -p meshc --test e2e_m033_s03 e2e_m033_s03_composed_reads_dashboard_aggregates -- --nocapture` | 101 | ❌ fail | 36970ms |


## Deviations

Instead of finishing the full T02 composed-read proof bundle, I stopped after isolating a proof-surface blocker in the copied storage-probe harness. I repaired the baseline harness, added partial T02 probes, and documented the blocker for replan rather than forcing more speculative probe rewrites.

## Known Issues

`compiler/meshc/tests/e2e_m033_s03.rs` now contains partial `composed_reads` work, but the remaining T02 proof path is blocked by current compiler/runtime behavior in temporary Mesher storage probes: imported `List<Issue>` / similar struct-list results can fail with LLVM verifier or runtime switch crashes, and some aggregate map rows stringify as raw pointer addresses when staged through helper bindings. The Mesher app itself still builds, but the storage-only proof strategy assumed by the task plan is not sufficient to finish T02 as written.

## Files Created/Modified

- `compiler/meshc/tests/e2e_m033_s03.rs`
- `mesher/storage/queries.mpl`
- `.gsd/KNOWLEDGE.md`
