---
id: T02
parent: S05
milestone: M051
provides: []
requires: []
affects: []
key_files: ["website/docs/docs/tooling/index.md", "website/docs/docs/distributed-proof/index.md", "scripts/tests/verify-m036-s03-contract.test.mjs", "scripts/tests/verify-m050-s02-first-contact-contract.test.mjs", "scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs", "compiler/meshc/tests/e2e_m051_s04.rs", "scripts/verify-m051-s04.sh", ".gsd/milestones/M051/slices/S05/tasks/T02-SUMMARY.md"]
key_decisions: ["Kept the public backend-proof handoff phrased in terms of Production Backend Proof, Mesher, and the retained verifier instead of any repo-root compatibility runbook.", "Updated the source contracts, Rust source rail, and built-html verifier together so stale public wording cannot survive through a docs build."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Ran the three task-level Node docs contracts, the Rust S04 contract target, and the slice-level verification excerpt commands. All seven checks passed after the wording and contract updates landed."
completed_at: 2026-04-04T21:49:08.436Z
blocker_discovered: false
---

# T02: Removed the last public `reference-backend` doc wording and locked the docs contracts to the generic backend-proof handoff.

> Removed the last public `reference-backend` doc wording and locked the docs contracts to the generic backend-proof handoff.

## What Happened
---
id: T02
parent: S05
milestone: M051
key_files:
  - website/docs/docs/tooling/index.md
  - website/docs/docs/distributed-proof/index.md
  - scripts/tests/verify-m036-s03-contract.test.mjs
  - scripts/tests/verify-m050-s02-first-contact-contract.test.mjs
  - scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs
  - compiler/meshc/tests/e2e_m051_s04.rs
  - scripts/verify-m051-s04.sh
  - .gsd/milestones/M051/slices/S05/tasks/T02-SUMMARY.md
key_decisions:
  - Kept the public backend-proof handoff phrased in terms of Production Backend Proof, Mesher, and the retained verifier instead of any repo-root compatibility runbook.
  - Updated the source contracts, Rust source rail, and built-html verifier together so stale public wording cannot survive through a docs build.
duration: ""
verification_result: passed
completed_at: 2026-04-04T21:49:08.437Z
blocker_discovered: false
---

# T02: Removed the last public `reference-backend` doc wording and locked the docs contracts to the generic backend-proof handoff.

**Removed the last public `reference-backend` doc wording and locked the docs contracts to the generic backend-proof handoff.**

## What Happened

Rewrote the public tooling page so the LSP/editor proof story is described against a small backend-shaped Mesh project instead of `reference-backend/`, and removed the stale same-file-definition wording pinned to `reference-backend/api/jobs.mpl`. Cleaned the distributed-proof page’s remaining deeper-backend bullet so it now points readers to Production Backend Proof, Mesher, and the retained verifier instead of naming the deleted repo-root compatibility tree. Tightened the M036, M050, and M051 contract rails around those exact removals: the tooling/editor contract now requires the generic backend-shaped proof wording and fails on reintroduced `reference-backend` markers; the first-contact contract now forbids the stale editor-proof path leak alongside the older repo-root backend commands; the secondary-surface contract now requires the new distributed-proof handoff sentence and fails if the old deeper-backend claim comes back. Updated the M051 S04 source and built-html verifier expectations so the public markdown, source contracts, and VitePress replay all agree on the new wording.

## Verification

Ran the three task-level Node docs contracts, the Rust S04 contract target, and the slice-level verification excerpt commands. All seven checks passed after the wording and contract updates landed.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `node --test scripts/tests/verify-m036-s03-contract.test.mjs` | 0 | ✅ pass | 1463ms |
| 2 | `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs` | 0 | ✅ pass | 855ms |
| 3 | `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` | 0 | ✅ pass | 580ms |
| 4 | `cargo test -p meshc --test e2e_m051_s04 -- --nocapture` | 0 | ✅ pass | 40921ms |
| 5 | `bash scripts/verify-production-proof-surface.sh` | 0 | ✅ pass | 5240ms |
| 6 | `cargo test -p meshc --test e2e_m050_s01 -- --nocapture` | 0 | ✅ pass | 5646ms |
| 7 | `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` | 0 | ✅ pass | 7008ms |


## Deviations

None.

## Known Issues

`cargo test` for the touched Rust rails still emits pre-existing unused-helper warnings from compiler/meshc/tests/support/*.rs. The verification results stayed green; this task did not change that warning baseline.

## Files Created/Modified

- `website/docs/docs/tooling/index.md`
- `website/docs/docs/distributed-proof/index.md`
- `scripts/tests/verify-m036-s03-contract.test.mjs`
- `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`
- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
- `compiler/meshc/tests/e2e_m051_s04.rs`
- `scripts/verify-m051-s04.sh`
- `.gsd/milestones/M051/slices/S05/tasks/T02-SUMMARY.md`


## Deviations
None.

## Known Issues
`cargo test` for the touched Rust rails still emits pre-existing unused-helper warnings from compiler/meshc/tests/support/*.rs. The verification results stayed green; this task did not change that warning baseline.
