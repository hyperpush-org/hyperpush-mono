---
id: T04
parent: S04
milestone: M051
provides: []
requires: []
affects: []
key_files: ["compiler/meshc/tests/e2e_m047_s05.rs", ".tmp/m050-s01/verify/m047-s04-docs-contract.log", ".tmp/m050-s02/verify/m047-s05-docs-contract.log", ".tmp/m050-s03/verify/m047-s04-docs-contract.log", ".gsd/milestones/M051/slices/S04/tasks/T04-SUMMARY.md"]
key_decisions: ["Treat the failing M047 historical docs tests as the immediate gating root cause for the M050 wrapper stack before adding the new M051/S04 replay.", "Stop at the context-budget boundary once the exact stale expectations were reproduced and one compile regression was fixed, instead of guessing broader historical-test rewrites without a fresh unit."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Verified the current state in two layers. `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`, `cargo test -p meshc --test e2e_m050_s02 -- --nocapture`, and `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` all passed, confirming the stale behavior is not in the M050 source-only matcher tests. The real wrapper replays then failed against live M047 rails, and targeted reruns confirmed the exact remaining drifts: `e2e_m047_s05` initially failed to compile because `distributed` was undefined; after fixing that, it failed on a stale Tooling expectation for `/docs/distributed-proof/`. `e2e_m047_s04` and `e2e_m047_s06` both fail because they still require Distributed Proof URLs in `README.md` and Tooling. The task-owned commands `cargo test -p meshc --test e2e_m051_s04 -- --nocapture` and `bash scripts/verify-m051-s04.sh` were not runnable because those assets are still unimplemented."
completed_at: 2026-04-04T19:16:31.985Z
blocker_discovered: false
---

# T04: Fixed the immediate `e2e_m047_s05` compile regression and captured the remaining stale M047/M050 docs-rail failures that still block the new M051/S04 replay.

> Fixed the immediate `e2e_m047_s05` compile regression and captured the remaining stale M047/M050 docs-rail failures that still block the new M051/S04 replay.

## What Happened
---
id: T04
parent: S04
milestone: M051
key_files:
  - compiler/meshc/tests/e2e_m047_s05.rs
  - .tmp/m050-s01/verify/m047-s04-docs-contract.log
  - .tmp/m050-s02/verify/m047-s05-docs-contract.log
  - .tmp/m050-s03/verify/m047-s04-docs-contract.log
  - .gsd/milestones/M051/slices/S04/tasks/T04-SUMMARY.md
key_decisions:
  - Treat the failing M047 historical docs tests as the immediate gating root cause for the M050 wrapper stack before adding the new M051/S04 replay.
  - Stop at the context-budget boundary once the exact stale expectations were reproduced and one compile regression was fixed, instead of guessing broader historical-test rewrites without a fresh unit.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T19:16:31.986Z
blocker_discovered: false
---

# T04: Fixed the immediate `e2e_m047_s05` compile regression and captured the remaining stale M047/M050 docs-rail failures that still block the new M051/S04 replay.

**Fixed the immediate `e2e_m047_s05` compile regression and captured the remaining stale M047/M050 docs-rail failures that still block the new M051/S04 replay.**

## What Happened

Activated the required bash-scripting, test, and VitePress skills, read the T04 plan plus the existing M050 wrappers and historical rails, and reproduced the live verifier state before editing. The M050 Rust source-contract targets all passed, which showed the remaining drift was in the real wrapper replays. Reproducing `bash scripts/verify-m050-s01.sh`, `bash scripts/verify-m050-s02.sh`, and `bash scripts/verify-m050-s03.sh` exposed the actual gate: the wrappers fail as soon as they hit older M047 historical docs rails. The first hard failure was a real compile regression in `compiler/meshc/tests/e2e_m047_s05.rs`, where the test asserted against `distributed` without loading that file; I fixed that by adding the missing archived `website/docs/docs/distributed/index.md` read. After that fix, the remaining failures are truthful stale expectations: `e2e_m047_s05` still expects `/docs/distributed-proof/` in Tooling, while `e2e_m047_s04` and `e2e_m047_s06` still require Distributed Proof URLs in `README.md` and Tooling even though S04 moved the public deeper handoff to Production Backend Proof. The context-budget warning forced wrap-up before I could finish those historical expectation rewrites or create `compiler/meshc/tests/e2e_m051_s04.rs` and `scripts/verify-m051-s04.sh`, so this task record captures the exact resume point.

## Verification

Verified the current state in two layers. `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`, `cargo test -p meshc --test e2e_m050_s02 -- --nocapture`, and `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` all passed, confirming the stale behavior is not in the M050 source-only matcher tests. The real wrapper replays then failed against live M047 rails, and targeted reruns confirmed the exact remaining drifts: `e2e_m047_s05` initially failed to compile because `distributed` was undefined; after fixing that, it failed on a stale Tooling expectation for `/docs/distributed-proof/`. `e2e_m047_s04` and `e2e_m047_s06` both fail because they still require Distributed Proof URLs in `README.md` and Tooling. The task-owned commands `cargo test -p meshc --test e2e_m051_s04 -- --nocapture` and `bash scripts/verify-m051-s04.sh` were not runnable because those assets are still unimplemented.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m050_s01 -- --nocapture` | 0 | ✅ pass | 28200ms |
| 2 | `cargo test -p meshc --test e2e_m050_s02 -- --nocapture` | 0 | ✅ pass | 28200ms |
| 3 | `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` | 0 | ✅ pass | 28200ms |
| 4 | `bash scripts/verify-m050-s01.sh` | 1 | ❌ fail | 12900ms |
| 5 | `bash scripts/verify-m050-s02.sh` | 1 | ❌ fail | 5200ms |
| 6 | `bash scripts/verify-m050-s03.sh` | 1 | ❌ fail | 12900ms |
| 7 | `cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture` | 101 | ❌ fail | 1300ms |
| 8 | `cargo test -p meshc --test e2e_m047_s04 m047_s04_ -- --nocapture` | 101 | ❌ fail | 1500ms |
| 9 | `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture` | 101 | ❌ fail | 1240ms |


## Deviations

I did not reach the planned M051/S04 replay implementation in this unit because the context-budget warning forced an early wrap-up after reproducing the live wrapper failures and fixing the first compile-blocking regression in `e2e_m047_s05.rs`.

## Known Issues

`compiler/meshc/tests/e2e_m047_s04.rs`, `compiler/meshc/tests/e2e_m047_s05.rs`, and `compiler/meshc/tests/e2e_m047_s06.rs` still encode stale public-surface expectations around Distributed Proof versus Production Backend Proof, which keeps `bash scripts/verify-m050-s01.sh`, `bash scripts/verify-m050-s02.sh`, and `bash scripts/verify-m050-s03.sh` red. `compiler/meshc/tests/e2e_m051_s04.rs` and `scripts/verify-m051-s04.sh` have not been created yet, so the task-owned verification commands remain unavailable.

## Files Created/Modified

- `compiler/meshc/tests/e2e_m047_s05.rs`
- `.tmp/m050-s01/verify/m047-s04-docs-contract.log`
- `.tmp/m050-s02/verify/m047-s05-docs-contract.log`
- `.tmp/m050-s03/verify/m047-s04-docs-contract.log`
- `.gsd/milestones/M051/slices/S04/tasks/T04-SUMMARY.md`


## Deviations
I did not reach the planned M051/S04 replay implementation in this unit because the context-budget warning forced an early wrap-up after reproducing the live wrapper failures and fixing the first compile-blocking regression in `e2e_m047_s05.rs`.

## Known Issues
`compiler/meshc/tests/e2e_m047_s04.rs`, `compiler/meshc/tests/e2e_m047_s05.rs`, and `compiler/meshc/tests/e2e_m047_s06.rs` still encode stale public-surface expectations around Distributed Proof versus Production Backend Proof, which keeps `bash scripts/verify-m050-s01.sh`, `bash scripts/verify-m050-s02.sh`, and `bash scripts/verify-m050-s03.sh` red. `compiler/meshc/tests/e2e_m051_s04.rs` and `scripts/verify-m051-s04.sh` have not been created yet, so the task-owned verification commands remain unavailable.
