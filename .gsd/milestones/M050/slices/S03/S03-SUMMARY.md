---
id: S03
parent: M050
milestone: M050
provides:
  - A single public-secondary clustered proof map on `/docs/distributed-proof/` with explicit SQLite-local vs Postgres-shared/reference-backend boundaries.
  - A compact `/docs/production-backend-proof/` handoff that supporting subsystem guides route through before the deeper backend runbook.
  - A dedicated retained S03 verifier bundle that the assembled M049 replay now runs and retains before older historical wrappers.
requires:
  - slice: S01
    provides: public proof pages demoted to secondary sidebar placement with footer opt-out and a fast docs-graph preflight
  - slice: S02
    provides: the scaffold/examples-first first-contact path plus the retained S02 verifier replayed in the assembled M049 wrapper
affects:
  - M051
  - M052
key_files:
  - website/docs/docs/distributed-proof/index.md
  - website/docs/docs/distributed/index.md
  - website/docs/docs/tooling/index.md
  - website/docs/docs/production-backend-proof/index.md
  - website/docs/docs/web/index.md
  - website/docs/docs/databases/index.md
  - website/docs/docs/testing/index.md
  - website/docs/docs/concurrency/index.md
  - scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs
  - reference-backend/scripts/verify-production-proof-surface.sh
  - scripts/verify-m050-s03.sh
  - compiler/meshc/tests/e2e_m050_s03.rs
  - scripts/verify-m049-s05.sh
  - scripts/tests/verify-m049-s05-contract.test.mjs
  - compiler/meshc/tests/e2e_m049_s05.rs
  - compiler/meshc/tests/e2e_m047_s04.rs
  - compiler/meshc/tests/e2e_m047_s05.rs
  - compiler/meshc/tests/e2e_m047_s06.rs
  - scripts/verify-m047-s04.sh
  - scripts/verify-m047-s05.sh
  - scripts/verify-m047-s06.sh
key_decisions:
  - Keep `Distributed Proof` as the only public-secondary clustered verifier map; `Distributed Actors` and `Tooling` keep bounded handoffs instead of duplicating the retained rail ledger.
  - Keep `Production Backend Proof` compact and backend-only, and separate its verifier seam from the broader cross-page role/routing contract.
  - Introduce `bash scripts/verify-m050-s03.sh` as a dedicated retained S03 verifier and replay it immediately after `bash scripts/verify-m050-s02.sh` inside `bash scripts/verify-m049-s05.sh`.
patterns_established:
  - Give each proof-oriented docs surface one role and one verifier seam: cross-page routing via a source contract test, page-specific compactness/parity via a focused verifier, and rendered truth via built-HTML assertions.
  - Retain dedicated docs verifier bundles and copy them into larger historical wrappers instead of reimplementing the same assertions in every assembled replay.
  - Validate VitePress proof pages from normalized `<main>` text plus explicit link markers rather than raw greps over emitted HTML.
observability_surfaces:
  - .tmp/m050-s03/verify/{status.txt,current-phase.txt,phase-report.txt,full-contract.log,latest-proof-bundle.txt}
  - `.tmp/m050-s03/verify/built-html/summary.json` plus copied `distributed`, `distributed-proof`, and `production-backend-proof` HTML snapshots
  - `.tmp/m049-s05/verify/retained-proof-bundle/retained-m050-s03-verify/` as the retained S03 bundle inside the assembled replay
drill_down_paths:
  - .gsd/milestones/M050/slices/S03/tasks/T01-SUMMARY.md
  - .gsd/milestones/M050/slices/S03/tasks/T02-SUMMARY.md
  - .gsd/milestones/M050/slices/S03/tasks/T03-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-04T04:46:02.463Z
blocker_discovered: false
---

# S03: Secondary Docs Surfaces & Two-Layer Truth

**Public secondary proof pages now have one clear ownership split: `Distributed Proof` is the canonical clustered proof map, `Production Backend Proof` is the compact backend handoff, and dedicated retained verifiers prove both the source routing and the built-site output before the heavier M049 replay runs.**

## What Happened

S03 finished the public-secondary role split that S01 and S02 set up. `website/docs/docs/distributed-proof/index.md` is now the only public-secondary page that carries the named clustered verifier rails, including the retained M047 cutover/closeout wrappers, the repo S07 clustered-route rail, the read-only Fly path, and the explicit SQLite-local vs Postgres-shared/reference-backend boundary. In parallel, `website/docs/docs/distributed/index.md` was pulled back to a primitives-first guide with bounded handoffs, and `website/docs/docs/tooling/index.md` now stops at the CLI/operator order before intentionally routing readers to `Clustered Example`, `Distributed Proof`, or `Production Backend Proof`.

The slice also made backend proof routing coherent without turning `Production Backend Proof` into another onboarding page. `website/docs/docs/production-backend-proof/index.md` stayed intentionally compact and backend-only, while `website/docs/docs/web/index.md`, `website/docs/docs/databases/index.md`, `website/docs/docs/testing/index.md`, and `website/docs/docs/concurrency/index.md` now all route through that page before the deeper `reference-backend/README.md` runbook. To keep failure seams honest, `reference-backend/scripts/verify-production-proof-surface.sh` now owns production-proof-page compactness, sidebar placement, command parity, recovery fields, and first-contact exclusions, while the new `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` owns cross-page role/routing drift.

Finally, S03 became a retained proof surface of its own. `scripts/verify-m050-s03.sh` replays the secondary-surface source contract, the retained M047 docs rails, the production backend proof verifier, one serial VitePress build, and built-HTML assertions for `distributed`, `distributed-proof`, and `production-backend-proof`, then retains the resulting `.tmp/m050-s03/verify` bundle with copied HTML snapshots plus `built-html/summary.json`. `compiler/meshc/tests/e2e_m050_s03.rs` pins that verifier’s phase order and bundle markers, and `scripts/verify-m049-s05.sh`, `scripts/tests/verify-m049-s05-contract.test.mjs`, and `compiler/meshc/tests/e2e_m049_s05.rs` now replay S03 immediately after the S02 preflight and retain `retained-m050-s03-verify` inside the assembled M049 bundle so docs-surface drift fails before older runtime-heavy retained rails can hide it.

## Verification

Passed all slice-plan verification rails and the retained assembled replay:

- `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
- `bash reference-backend/scripts/verify-production-proof-surface.sh`
- `cargo test -p meshc --test e2e_m050_s03 -- --nocapture`
- `node --test scripts/tests/verify-m049-s05-contract.test.mjs`
- `cargo test -p meshc --test e2e_m049_s05 -- --nocapture`
- `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`
- `cargo test -p meshc --test e2e_m047_s05 -- --nocapture`
- `cargo test -p meshc --test e2e_m047_s06 -- --nocapture`
- `bash scripts/verify-m050-s03.sh`
- `bash scripts/verify-m049-s05.sh`

Artifact checks after the green replays confirmed the diagnostic surfaces themselves work:
- `.tmp/m050-s03/verify/phase-report.txt` shows `secondary-surfaces-contract`, `m047-s04-docs-contract`, `m047-s05-docs-contract`, `m047-s06-docs-contract`, `production-proof-surface`, `docs-build`, `retain-built-html`, `built-html`, and `m050-s03-bundle-shape` all passed.
- `.tmp/m050-s03/verify/built-html/summary.json` captured normalized marker/link maps for `distributed`, `distributed-proof`, and `production-backend-proof`.
- `.tmp/m049-s05/verify/phase-report.txt` shows `m050-s03-preflight` passed immediately after `m050-s02-preflight`, and `.tmp/m049-s05/verify/retained-proof-bundle/retained-m050-s03-verify/phase-report.txt` retained the full S03 phase history.

## Requirements Advanced

None.

## Requirements Validated

- R117 — Validated by the completed M050 docs-reset chain: `bash scripts/verify-m050-s01.sh`, `bash scripts/verify-m050-s02.sh`, `bash scripts/verify-m050-s03.sh`, `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`, `bash reference-backend/scripts/verify-production-proof-surface.sh`, and the green assembled replay `bash scripts/verify-m049-s05.sh` prove that public docs stay evaluator-facing while deeper proof maps remain secondary.
- R118 — Validated by the completed M050 split: `bash scripts/verify-m050-s02.sh`, `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`, `bash scripts/verify-m050-s03.sh`, and the retained `bash scripts/verify-m049-s05.sh` replay prove that `Clustered Example` stays the primary evaluator path while `Distributed Actors`, `Distributed Proof`, and `Production Backend Proof` now have distinct roles.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

None.

## Known Limitations

- `Distributed Proof` still intentionally exposes repo-level named verifier commands and historical alias rails; S03 centralized that proof map, but it did not remove the deeper retained proof material from the public-secondary docs.
- `Production Backend Proof` still points at `reference-backend/README.md` as the deeper backend runbook until M051 replaces that reference surface.
- The current GSD requirements DB still rejects `R117` and `R118` as `not found`, so the authoritative visible requirement state for M050 currently lives in the checked-in `.gsd/REQUIREMENTS.md` plus the saved requirement decisions.

## Follow-ups

- M051 should repoint `Production Backend Proof` and its verifier from `reference-backend` to `mesher` without letting backend-proof detail leak back into the first-contact docs path.
- M052 should carry the same primary/secondary role split into the landing and packages surfaces so the evaluator-facing story stays coherent outside the docs site too.

## Files Created/Modified

- `website/docs/docs/distributed-proof/index.md` — Made Distributed Proof the only public-secondary clustered proof map, with retained M047 rails, S07 route discoverability, Fly read-only guidance, and the explicit SQLite/Postgres/reference-backend split.
- `website/docs/docs/distributed/index.md` — Reframed the distributed guide around low-level primitives with bounded handoffs to Clustered Example, Distributed Proof, and Production Backend Proof.
- `website/docs/docs/tooling/index.md` — Trimmed the clustered proof section to the runtime-owned CLI/operator order and routed deeper readers to the appropriate proof surfaces instead of duplicating the clustered rail ledger.
- `website/docs/docs/production-backend-proof/index.md` — Kept the backend proof page compact and canonical, with command parity, recovery fields, generic-guide routing, and explicit first-contact exclusions.
- `website/docs/docs/web/index.md` — Added a clickable handoff through Production Backend Proof before the deeper backend runbook.
- `website/docs/docs/databases/index.md` — Added a clickable handoff through Production Backend Proof before the deeper backend runbook.
- `website/docs/docs/testing/index.md` — Added a clickable handoff through Production Backend Proof before the deeper backend runbook.
- `website/docs/docs/concurrency/index.md` — Added a clickable handoff through Production Backend Proof before the deeper backend runbook.
- `reference-backend/scripts/verify-production-proof-surface.sh` — Pinned the production proof page’s compact role, sidebar placement, runbook parity, recovery vocabulary, and first-contact exclusions.
- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` — Added the S03 source contract for cross-page role ownership and routing across distributed, tooling, backend-proof, and supporting subsystem guides.
- `scripts/verify-m050-s03.sh` — Added the dedicated retained S03 verifier with source-contract, retained M047 docs rails, production-proof-surface replay, serial docs build, built-HTML assertions, and bundle-shape checks.
- `compiler/meshc/tests/e2e_m050_s03.rs` — Pinned the S03 verifier’s phase order, retained HTML snapshots, and bundle markers at the Rust contract layer.
- `scripts/verify-m049-s05.sh` — Inserted the S03 preflight immediately after S02 and retained the resulting `retained-m050-s03-verify` bundle inside the assembled M049 replay.
- `scripts/tests/verify-m049-s05-contract.test.mjs` — Updated the assembled replay contract to require the new S03 preflight order and retained bundle markers.
- `compiler/meshc/tests/e2e_m049_s05.rs` — Updated the Rust contract for the assembled M049 replay to require the S03 preflight and retained bundle markers.
- `compiler/meshc/tests/e2e_m047_s04.rs` — Retargeted the retained M047 S04 docs contract to the new Distributed Proof ownership split.
- `compiler/meshc/tests/e2e_m047_s05.rs` — Retargeted the retained M047 S05 docs contract to the new Distributed Proof ownership split and non-canonical page boundaries.
- `compiler/meshc/tests/e2e_m047_s06.rs` — Retargeted the retained M047 S06 docs/closeout contract to the new secondary-surface ownership split.
- `scripts/verify-m047-s04.sh` — Aligned the historical S04 shell rail with the new Distributed Proof ownership and lighter non-canonical docs surfaces.
- `scripts/verify-m047-s05.sh` — Aligned the historical S05 shell rail with the new Distributed Proof ownership and lighter non-canonical docs surfaces.
- `scripts/verify-m047-s06.sh` — Aligned the historical S06 shell rail with the new Distributed Proof ownership and lighter non-canonical docs surfaces.
- `.gsd/KNOWLEDGE.md` — Recorded the S03 regression-debug path and the current M050 requirement-DB mismatch for future closers.
- `.gsd/PROJECT.md` — Refreshed the project state to reflect completed M050/S03 docs and verifier work.
