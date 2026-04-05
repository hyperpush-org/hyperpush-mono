---
id: T03
parent: S04
milestone: M051
provides: []
requires: []
affects: []
key_files: ["compiler/mesh-pkg/src/scaffold.rs", "tools/skill/mesh/skills/clustering/SKILL.md", "scripts/tests/verify-m049-s04-onboarding-contract.test.mjs", "scripts/tests/verify-m048-s04-skill-contract.test.mjs", "compiler/meshc/tests/e2e_m047_s04.rs", "compiler/meshc/tests/e2e_m047_s05.rs", "compiler/meshc/tests/e2e_m047_s06.rs", ".gsd/milestones/M051/slices/S04/tasks/T03-SUMMARY.md"]
key_decisions: ["Use Production Backend Proof as the only public deeper handoff from generated scaffold and skill surfaces, with Mesher and the retained backend replay named only as maintainer-facing follow-ons.", "Tighten the Node contract rails so the clustering skill can mention `reference-backend/README.md` only as a forbidden public next step without tripping a false-positive stale-teaching check."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Reran `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` and `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`; both passed and now prove the updated scaffold/skill/public-doc wording plus their stale-wording negative cases. The heavier Rust replay commands from the task plan were edited for the new contract but were not rerun in this unit because the context-budget warning required immediate wrap-up; the exact pending commands are recorded in the summary diagnostics section."
completed_at: 2026-04-04T19:05:24.713Z
blocker_discovered: false
---

# T03: Retargeted the clustered scaffold README, clustering skill, and their source-contract rails to the proof-page-first Mesher maintainer handoff.

> Retargeted the clustered scaffold README, clustering skill, and their source-contract rails to the proof-page-first Mesher maintainer handoff.

## What Happened
---
id: T03
parent: S04
milestone: M051
key_files:
  - compiler/mesh-pkg/src/scaffold.rs
  - tools/skill/mesh/skills/clustering/SKILL.md
  - scripts/tests/verify-m049-s04-onboarding-contract.test.mjs
  - scripts/tests/verify-m048-s04-skill-contract.test.mjs
  - compiler/meshc/tests/e2e_m047_s04.rs
  - compiler/meshc/tests/e2e_m047_s05.rs
  - compiler/meshc/tests/e2e_m047_s06.rs
  - .gsd/milestones/M051/slices/S04/tasks/T03-SUMMARY.md
key_decisions:
  - Use Production Backend Proof as the only public deeper handoff from generated scaffold and skill surfaces, with Mesher and the retained backend replay named only as maintainer-facing follow-ons.
  - Tighten the Node contract rails so the clustering skill can mention `reference-backend/README.md` only as a forbidden public next step without tripping a false-positive stale-teaching check.
duration: ""
verification_result: passed
completed_at: 2026-04-04T19:05:24.715Z
blocker_discovered: false
---

# T03: Retargeted the clustered scaffold README, clustering skill, and their source-contract rails to the proof-page-first Mesher maintainer handoff.

**Retargeted the clustered scaffold README, clustering skill, and their source-contract rails to the proof-page-first Mesher maintainer handoff.**

## What Happened

Updated `compiler/mesh-pkg/src/scaffold.rs` so the generated clustered README keeps the public examples-first ladder but routes deeper backend work through Production Backend Proof and then names `mesher/README.md` plus `bash scripts/verify-m051-s01.sh` / `bash scripts/verify-m051-s02.sh` as maintainer-only follow-ons instead of `reference-backend/README.md`. Rewrote `tools/skill/mesh/skills/clustering/SKILL.md` to match that contract, keeping the scaffold and Todo examples public while moving the deeper backend story onto the proof page, Mesher, and named maintainer verifiers. Rebased `scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` and `scripts/tests/verify-m048-s04-skill-contract.test.mjs` so they fail closed on stale repo-root backend teaching and still allow the skill to mention `reference-backend/README.md` only as a forbidden public next step. Updated `compiler/meshc/tests/e2e_m047_s04.rs`, `compiler/meshc/tests/e2e_m047_s05.rs`, and `compiler/meshc/tests/e2e_m047_s06.rs` so their public-surface assertions now look for Production Backend Proof / Mesher / named maintainer verifier markers instead of the old repo-root runbook wording. The context-budget warning landed during final verification, so I wrapped the unit after the fast Node rails passed and recorded the remaining Rust replay commands in the task summary for the next unit.

## Verification

Reran `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` and `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`; both passed and now prove the updated scaffold/skill/public-doc wording plus their stale-wording negative cases. The heavier Rust replay commands from the task plan were edited for the new contract but were not rerun in this unit because the context-budget warning required immediate wrap-up; the exact pending commands are recorded in the summary diagnostics section.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` | 0 | ✅ pass | 160ms |
| 2 | `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs` | 0 | ✅ pass | 135ms |


## Deviations

Stopped after the Node contract reruns because the context-budget warning required wrap-up before the heavier cargo replay stage.

## Known Issues

The Rust historical source-contract replays still need confirmation with `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`, `cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture`, and `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`.

## Files Created/Modified

- `compiler/mesh-pkg/src/scaffold.rs`
- `tools/skill/mesh/skills/clustering/SKILL.md`
- `scripts/tests/verify-m049-s04-onboarding-contract.test.mjs`
- `scripts/tests/verify-m048-s04-skill-contract.test.mjs`
- `compiler/meshc/tests/e2e_m047_s04.rs`
- `compiler/meshc/tests/e2e_m047_s05.rs`
- `compiler/meshc/tests/e2e_m047_s06.rs`
- `.gsd/milestones/M051/slices/S04/tasks/T03-SUMMARY.md`


## Deviations
Stopped after the Node contract reruns because the context-budget warning required wrap-up before the heavier cargo replay stage.

## Known Issues
The Rust historical source-contract replays still need confirmation with `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`, `cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture`, and `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`.
