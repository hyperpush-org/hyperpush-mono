---
id: S04
parent: M051
milestone: M051
provides:
  - A coherent public examples-first docs/scaffold/skill story that no longer hands readers directly to `reference-backend/README.md`.
  - A maintained deeper-backend handoff contract naming Mesher and the retained backend replay only through `/docs/production-backend-proof/`.
  - One authoritative assembled S04 verifier and retained proof bundle for downstream S05 deletion and closeout work.
requires:
  - slice: S01
    provides: Mesher maintainer runbook and `bash scripts/verify-m051-s01.sh` as the deeper maintained app surface.
  - slice: S02
    provides: Retained backend-only maintainer verifier (`bash scripts/verify-m051-s02.sh`) and the compatibility proof-page path that S04 retargeted.
  - slice: S03
    provides: The retained backend-shaped tooling/editor fixture and wrapper expectations that had to remain aligned with the new public docs story.
affects:
  - S05
key_files:
  - README.md
  - website/docs/docs/getting-started/index.md
  - website/docs/docs/getting-started/clustered-example/index.md
  - website/docs/docs/tooling/index.md
  - website/docs/docs/distributed/index.md
  - website/docs/docs/distributed-proof/index.md
  - website/docs/docs/production-backend-proof/index.md
  - compiler/mesh-pkg/src/scaffold.rs
  - tools/skill/mesh/skills/clustering/SKILL.md
  - scripts/tests/verify-m049-s04-onboarding-contract.test.mjs
  - scripts/tests/verify-m048-s04-skill-contract.test.mjs
  - scripts/tests/verify-m048-s05-contract.test.mjs
  - scripts/tests/verify-m050-s02-first-contact-contract.test.mjs
  - scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs
  - compiler/meshc/tests/e2e_m047_s04.rs
  - compiler/meshc/tests/e2e_m047_s05.rs
  - compiler/meshc/tests/e2e_m047_s06.rs
  - compiler/meshc/tests/e2e_m051_s04.rs
  - scripts/verify-m050-s02.sh
  - scripts/verify-m050-s03.sh
  - scripts/verify-m051-s04.sh
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Keep `/docs/production-backend-proof/` as the compact public-secondary backend handoff instead of replacing the route, but make Mesher and the retained backend replay maintainer-only follow-ons behind that page.
  - Treat the retained M047 and M050 docs/wrapper rails as part of the slice acceptance surface; update them in lockstep with public copy so stale historical expectations fail closed instead of silently diverging.
  - Create `compiler/meshc/tests/e2e_m051_s04.rs` plus `scripts/verify-m051-s04.sh` as the authoritative S04 acceptance surface that aggregates contracts, wrappers, and built-html evidence into one retained bundle.
patterns_established:
  - Public docs stay scaffold/examples-first; deeper backend work must go through `/docs/production-backend-proof/` before naming maintainer-only Mesher or retained backend replay surfaces.
  - Historical compatibility paths can stay alive during a cutover, but their contracts must be rewritten to the new truth instead of preserving stale wording.
  - Slice-owned shell replays should archive wrapper bundles, built-html snapshots, and slice artifacts together under one `.tmp/<slice>/verify/` root so downstream deletion or reassessment work has one stable evidence bundle.
observability_surfaces:
  - `.tmp/m051-s04/verify/status.txt`
  - `.tmp/m051-s04/verify/current-phase.txt`
  - `.tmp/m051-s04/verify/phase-report.txt`
  - `.tmp/m051-s04/verify/full-contract.log`
  - `.tmp/m051-s04/verify/latest-proof-bundle.txt`
  - `.tmp/m051-s04/verify/built-html/summary.json`
  - `.tmp/m051-s04/verify/retained-proof-bundle/`
drill_down_paths:
  - .gsd/milestones/M051/slices/S04/tasks/T01-SUMMARY.md
  - .gsd/milestones/M051/slices/S04/tasks/T02-SUMMARY.md
  - .gsd/milestones/M051/slices/S04/tasks/T03-SUMMARY.md
  - .gsd/milestones/M051/slices/S04/tasks/T04-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-04T19:48:25.824Z
blocker_discovered: false
---

# S04: Retarget public docs, scaffold, and skills to the examples-first story

**Retargeted public docs, scaffold output, bundled skills, and retained verifier rails so public readers land on scaffold/examples-first surfaces while Mesher and the retained backend replay are maintainer-only deeper handoffs.**

## What Happened

S04 finished the public-story cutover that M050 and the earlier M051 slices set up but had not fully closed. The slice verified that the top-level first-contact path was already largely examples-first, then rewrote the remaining public-secondary docs, generated scaffold guidance, bundled clustering skill, historical M047 docs rails, and M050 wrapper verifiers so they all speak the same current contract: public readers start with hello world, `meshc init --clustered`, and the checked-in Todo examples; when deeper backend proof is needed, they go through `/docs/production-backend-proof/`; and only maintainers continue from there into `mesher/README.md`, `bash scripts/verify-m051-s01.sh`, and `bash scripts/verify-m051-s02.sh`. The slice also created a new slice-owned acceptance target (`compiler/meshc/tests/e2e_m051_s04.rs`) and the authoritative assembled replay (`scripts/verify-m051-s04.sh`) that now composes the onboarding/skill contracts, the retained M050 wrapper stack, the historical M047 docs rails, the proof-page compatibility verifier, and a real VitePress build into one retained `.tmp/m051-s04/verify/` proof bundle.

The execution work had two distinct halves. First, public-source surfaces were retargeted: `website/docs/docs/production-backend-proof/index.md` now stays a compact public-secondary handoff instead of a repo-root backend runbook; `distributed`, `distributed-proof`, `getting-started`, `clustered-example`, `tooling`, the scaffold README template in `compiler/mesh-pkg/src/scaffold.rs`, and `tools/skill/mesh/skills/clustering/SKILL.md` all now keep the examples-first ladder explicit while naming Mesher and the retained backend replay only as maintainer-facing follow-ons. Second, the retained proof rails were reconciled to that story: `compiler/meshc/tests/e2e_m047_s04.rs`, `e2e_m047_s05.rs`, and `e2e_m047_s06.rs` were updated to stop requiring the old README/tooling/distributed-proof linkage shape; `scripts/tests/verify-m048-s05-contract.test.mjs`, `scripts/verify-m050-s02.sh`, and `scripts/verify-m050-s03.sh` were adjusted so their built-html assertions match the shipped public copy instead of stale repo-root backend expectations; and `scripts/verify-m051-s04.sh` now archives the M050 wrapper bundles, copied M051 slice artifacts, and built HTML snapshots needed by S05.

The result is one coherent public story and one coherent maintainer story. Public docs and generated guidance now send readers to scaffold output plus `/examples`, not to `reference-backend/README.md`. The old compatibility path `reference-backend/scripts/verify-production-proof-surface.sh` remains alive, but it now guards the new contract rather than the old runbook teaching. Downstream S05 can now delete the repo-root `reference-backend/` tree against a stable acceptance rail instead of trying to infer whether a broken wrapper failure is a real docs regression or just leftover stale wording.

## Verification

All slice-plan verification checks passed on the live tree. Passed source contracts: `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs`, `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`, `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`, and `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`. Passed retained historical docs rails: `cargo test -p meshc --test e2e_m047_s04 m047_s04_ -- --nocapture`, `cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture`, and `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`. Passed compatibility proof-page verifier: `bash reference-backend/scripts/verify-production-proof-surface.sh`. Passed slice-owned acceptance: `cargo test -p meshc --test e2e_m051_s04 -- --nocapture` and `bash scripts/verify-m051-s04.sh`. The assembled S04 replay also reran `bash scripts/verify-m050-s01.sh`, `bash scripts/verify-m050-s02.sh`, `bash scripts/verify-m050-s03.sh`, and `npm --prefix website run build`, and published the retained bundle under `.tmp/m051-s04/verify/retained-proof-bundle`.

## Requirements Advanced

- R008 — Kept the public documentation/examples story honest by routing readers through scaffold/examples-first surfaces and a compact backend proof handoff instead of stale repo-root backend teaching.

## Requirements Validated

None.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

T01 required no source edits because the live first-contact docs already matched the planned examples-first wording. During closeout, the slice also had to reconcile retained M047/M048/M050 contract and built-html assertions that still encoded stale README/tooling/proof-page expectations; those fixes were necessary to make the slice-owned acceptance rail truthful.

## Known Limitations

The retained docs and wrapper rails remain exact-string-sensitive by design. Future wording changes to README/tooling/proof pages must update the paired Node/Rust/shell contract checks in the same change or the historical wrappers will fail closed even when the visible docs look correct.

## Follow-ups

S05 can now delete the repo-root `reference-backend/` package and retarget any surviving compatibility surfaces against the stable S04 acceptance rail. If public wording changes again, update the retained M047/M050/M051 contract tests and built-html checks together instead of patching only the visible Markdown.

## Files Created/Modified

- `README.md` — Kept the public next-step ladder examples-first and limited the deeper backend handoff to Production Backend Proof plus maintainer-only Mesher language.
- `website/docs/docs/getting-started/index.md` — Kept the three-way starter chooser explicit and ordered the follow-on path to Clustered Example, Todo examples, then Production Backend Proof.
- `website/docs/docs/getting-started/clustered-example/index.md` — Kept the clustered scaffold tutorial public and examples-first while routing retained verifier details through Distributed Proof and deeper backend work through Production Backend Proof.
- `website/docs/docs/tooling/index.md` — Removed stale repo-root backend day-one commands, preserved the public CLI ladder, and clarified the editor/tooling proof surfaces.
- `website/docs/docs/distributed/index.md` — Retargeted backend-specific handoffs through Distributed Proof and Production Backend Proof, with Mesher named only as the maintainer-facing deeper app.
- `website/docs/docs/distributed-proof/index.md` — Reframed the page as the public-secondary verifier map that points to Production Backend Proof, Mesher, and the retained backend replay instead of a repo-root backend runbook.
- `website/docs/docs/production-backend-proof/index.md` — Rewrote the proof page as the compact public-secondary backend handoff for Mesher and the retained maintainer-only verifier surfaces.
- `compiler/mesh-pkg/src/scaffold.rs` — Updated the clustered scaffold README template to keep the examples-first ladder and the proof-page-first maintainer handoff.
- `tools/skill/mesh/skills/clustering/SKILL.md` — Retargeted the bundled clustering skill to the examples-first story with maintainer-only Mesher and retained backend follow-ons.
- `scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` — Pinned the generated scaffold and public onboarding wording to the new proof-page-first maintainer handoff.
- `scripts/tests/verify-m048-s04-skill-contract.test.mjs` — Pinned the clustering skill bundle to the examples-first public story and the maintainer-only deeper backend handoff.
- `scripts/tests/verify-m048-s05-contract.test.mjs` — Updated the retained public-tooling contract to match the current README and VS Code README wording.
- `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs` — Kept the first-contact docs contract aligned to the examples-first ladder and maintainer-only Mesher references.
- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` — Pinned secondary public docs to the Production Backend Proof plus Mesher/retained-verifier story.
- `compiler/meshc/tests/e2e_m047_s04.rs` — Updated the retained cutover docs rail to the current README/tooling/proof-page linkage expectations.
- `compiler/meshc/tests/e2e_m047_s05.rs` — Updated the retained clustered public-surface rail so README/tooling expectations match the current examples-first docs story.
- `compiler/meshc/tests/e2e_m047_s06.rs` — Updated the retained closeout docs rail to the current public docs hierarchy and proof-page handoff.
- `compiler/meshc/tests/e2e_m051_s04.rs` — Added the slice-owned S04 contract target covering docs, scaffold, skills, wrappers, and verifier ownership.
- `scripts/verify-m050-s02.sh` — Adjusted built-html first-contact checks to the shipped examples-first tooling story.
- `scripts/verify-m050-s03.sh` — Adjusted built-html secondary-surface checks to the shipped Mesher/Production Backend Proof contract.
- `scripts/verify-m051-s04.sh` — Added the authoritative assembled S04 replay that composes contracts, retained wrappers, docs build, and proof-bundle retention.
- `.gsd/KNOWLEDGE.md` — Recorded the retained-rails gotcha about README vs distributed/tooling linkage expectations and built-html validation scope.
