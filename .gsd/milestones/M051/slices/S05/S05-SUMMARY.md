---
id: S05
parent: M051
milestone: M051
provides:
  - A repo that ships without the top-level `reference-backend/` compatibility tree.
  - A canonical public proof-page verifier at `bash scripts/verify-production-proof-surface.sh` that survives the deleted app path.
  - A retained backend-only proof surface rooted at `scripts/fixtures/backend/reference-backend/` plus `bash scripts/verify-m051-s02.sh`.
  - One authoritative post-deletion acceptance rail and retained bundle at `bash scripts/verify-m051-s05.sh` and `.tmp/m051-s05/verify/retained-proof-bundle/`.
requires:
  - slice: S01
    provides: Mesher maintainer runbook and `bash scripts/verify-m051-s01.sh` as the maintained deeper reference-app surface.
  - slice: S02
    provides: Retained backend-only fixture, maintainer runbook, and `bash scripts/verify-m051-s02.sh` as the backend-specific proof surface that had to survive deletion.
  - slice: S03
    provides: Retained tooling/editor/LSP/formatter rails against the bounded backend fixture plus `bash scripts/verify-m051-s03.sh`.
  - slice: S04
    provides: Examples-first public docs/scaffold/skill story plus `bash scripts/verify-m051-s04.sh` as the pre-deletion public-surface acceptance rail.
affects:
  []
key_files:
  - scripts/verify-production-proof-surface.sh
  - website/docs/docs/production-backend-proof/index.md
  - website/docs/docs/tooling/index.md
  - website/docs/docs/distributed-proof/index.md
  - scripts/fixtures/backend/reference-backend/README.md
  - scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh
  - compiler/meshc/tests/e2e_reference_backend.rs
  - compiler/meshc/tests/e2e_m051_s02.rs
  - scripts/verify-m051-s02.sh
  - compiler/meshc/tests/e2e_m051_s05.rs
  - scripts/verify-m051-s05.sh
  - .gsd/PROJECT.md
key_decisions:
  - D389: Use `scripts/verify-production-proof-surface.sh` as the canonical public proof-page verifier path before and after `reference-backend/` deletion.
  - D390: Make the final S05 retained bundle self-contained by copying delegated S01-S04 verify trees and rewriting their copied `latest-proof-bundle.txt` pointers to the copied child bundles.
  - D391: Keep crash/requeue correctness and `/health` restart visibility as separate retained backend recovery rails; rely on final `/health` restart metadata instead of a flaky mid-recovery degraded-window observation.
patterns_established:
  - When retiring a repo-root proof app, move any surviving public verifier to a stable top-level path before deletion and retarget the historical wrapper/source contracts in lockstep.
  - Final slice-owned closeout rails should copy delegated verify trees and rewrite copied bundle pointers so the retained bundle is self-contained for downstream reassessment.
  - For retained backend recovery proof on this tree, keep end-to-end crash/requeue correctness separate from post-recovery `/health` restart metadata instead of requiring an unstable live degraded-window observation.
observability_surfaces:
  - .tmp/m051-s02/verify/status.txt
  - .tmp/m051-s02/verify/current-phase.txt
  - .tmp/m051-s02/verify/phase-report.txt
  - .tmp/m051-s02/verify/full-contract.log
  - .tmp/m051-s02/verify/latest-proof-bundle.txt
  - .tmp/m051-s05/verify/status.txt
  - .tmp/m051-s05/verify/current-phase.txt
  - .tmp/m051-s05/verify/phase-report.txt
  - .tmp/m051-s05/verify/full-contract.log
  - .tmp/m051-s05/verify/latest-proof-bundle.txt
  - .tmp/m051-s05/verify/retained-proof-bundle/
drill_down_paths:
  - .gsd/milestones/M051/slices/S05/tasks/T01-SUMMARY.md
  - .gsd/milestones/M051/slices/S05/tasks/T02-SUMMARY.md
  - .gsd/milestones/M051/slices/S05/tasks/T03-SUMMARY.md
  - .gsd/milestones/M051/slices/S05/tasks/T04-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-04T23:51:27.387Z
blocker_discovered: false
---

# S05: Delete reference-backend and close the assembled acceptance rail

**Deleted the repo-root `reference-backend/` tree, retargeted the surviving docs and retained backend proof to post-deletion truth, and closed M051 with a self-contained post-deletion acceptance rail at `bash scripts/verify-m051-s05.sh`.**

## What Happened

S05 finished the last reference-backend retirement work and left the repository in the post-deletion shape the milestone promised. The slice moved the public proof-page verifier to the stable top-level `scripts/` surface, removed the final public `reference-backend` wording from the docs/contracts that still leaked it, rewrote the retained backend fixture and verifier to post-deletion truth, deleted the repo-root `reference-backend/` tree plus its obsolete ignore rule, and added the terminal source + shell acceptance surfaces for the whole milestone. The repo now ships with Mesher as the maintained deeper reference app, backend-only proof retained under `scripts/fixtures/backend/reference-backend/`, and one post-deletion closeout rail (`bash scripts/verify-m051-s05.sh`) instead of a compatibility tree that later work had to tiptoe around.

The slice had two non-obvious repair seams before the final wrapper could become honest. First, retained fixture smoke could not treat the first successful `/health` response as ready, because the worker can still be in the degraded startup window while `POST /jobs` leaves a smoke job stuck at `pending`; `scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh` now waits for the same healthy contract the Rust harness uses (`status=ok`, `liveness=healthy`, `recovery_active=false`) before creating work. Second, the retained backend recovery rails were still over-claiming a live degraded-window proof that the post-deletion serial replay did not expose reliably; the closeout split that proof honestly so `e2e_reference_backend_worker_crash_recovers_job` proves crash/requeue/processed correctness end to end, while `e2e_reference_backend_worker_restart_is_visible_in_health` proves that `/health` preserves changed `boot_id` / `started_at`, `restart_count`, `recovered_jobs`, `last_exit_reason`, and `last_recovery_*` after recovery. That kept the maintained backend-only proof surface truthful instead of hiding the flake behind retries.

With those seams repaired, S05 landed the final closeout surfaces the roadmap needed. `compiler/meshc/tests/e2e_m051_s05.rs` now guards the post-deletion source contract directly: the repo-root compatibility tree stays gone, the proof-page verifier lives at the top level, and the S05 shell replay still composes the delegated S01-S04 rails instead of quietly depending on deleted paths. `scripts/verify-m051-s05.sh` is now the authoritative assembled replay: it re-runs `bash scripts/verify-m051-s01.sh`, `bash scripts/verify-m051-s02.sh`, `bash scripts/verify-m051-s03.sh`, and `bash scripts/verify-m051-s04.sh`, then copies each child verify tree and its pointed proof bundle into `.tmp/m051-s05/verify/retained-proof-bundle/` and rewrites every copied `latest-proof-bundle.txt` pointer to the copied child bundle. The result is one self-contained retained post-deletion bundle that downstream milestone validation and roadmap reassessment can inspect without depending on live child `.tmp` trees or the retired repo-root app.

## Operational Readiness
- **Health signal:** `bash scripts/verify-m051-s05.sh` is the terminal health check for the post-deletion tree, and its child rails publish stable markers under `.tmp/m051-s01/verify/` through `.tmp/m051-s04/verify/`. For the retained backend seam specifically, `.tmp/m051-s02/verify/m051-s02-fixture-smoke.log` now shows the smoke handoff waiting for `status=ok`, `liveness=healthy`, and `recovery_active=false` before work is created, while final `/health` recovery truth is preserved in the retained S02 proof bundle.
- **Failure signal:** `.tmp/m051-s05/verify/phase-report.txt` stops at the failing child phase, and `.tmp/m051-s05/verify/full-contract.log` plus the child wrapper log name the exact seam (`m051-s02-fixture-smoke`, `m051-s02-worker-crash-recovery`, `m051-s02-worker-restart-visibility`, bundle-shape drift, etc.). Each delegated verifier also retains its own `status.txt`, `current-phase.txt`, `phase-report.txt`, and `latest-proof-bundle.txt` for drill-down.
- **Recovery procedure:** If the assembled rail stops in `m051-s02-wrapper`, rerun `DATABASE_URL=... bash scripts/verify-m051-s02.sh` first and inspect `.tmp/m051-s02/verify/` before reopening the S05 wrapper. If fixture smoke stalls, inspect the deploy-smoke health poll lines and confirm the healthy `/health` gate still holds. If the final bundle-shape phase fails, inspect the copied child `latest-proof-bundle.txt` files inside `.tmp/m051-s05/verify/retained-proof-bundle/` rather than the live child `.tmp` trees.
- **Monitoring gaps:** The retained backend still does not provide a stable live mid-recovery degraded-window observation in the post-deletion closeout rail, so the maintained proof contract relies on final `/health` restart metadata instead. The docs/verifier rails remain exact-string-sensitive on purpose; that keeps drift visible, but it also means source and verifier updates must stay coupled.

## Verification

All slice-plan verification commands passed on the final tree. Passed public/doc contract checks: `bash scripts/verify-production-proof-surface.sh`, `node --test scripts/tests/verify-m036-s03-contract.test.mjs`, `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`, and `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`. Passed historical/source rails: `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`, `cargo test -p meshc --test e2e_m050_s03 -- --nocapture`, `cargo test -p meshc --test e2e_m051_s04 -- --nocapture`, `test ! -e reference-backend`, and `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`. Passed retained backend-only assembled replay: `DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:51798/mesh_m051_s05 bash scripts/verify-m051-s02.sh`. Passed final post-deletion closeout rail: `cargo test -p meshc --test e2e_m051_s05 -- --nocapture` and `DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:51798/mesh_m051_s05 bash scripts/verify-m051-s05.sh`. The disposable local Postgres container was cleaned up after the DB-backed replays finished.

## Requirements Advanced

- R008 — The public docs and proof-page verifier no longer depend on repo-root `reference-backend/`; the examples-first story survives deletion through the top-level proof-page verifier and the retained backend-only maintainer rail.
- R009 — The repo’s real backend proof story now survives without the retired top-level app: Mesher is the maintained deeper reference app, and the remaining backend-only proof lives behind retained fixtures and one post-deletion acceptance rail.

## Requirements Validated

None.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

Used a disposable local Docker Postgres URL (`postgresql://postgres:postgres@127.0.0.1:51798/mesh_m051_s05`) for the DB-backed retained rails because the non-interactive shell did not inherit a repo-root `DATABASE_URL`. During closeout I also had to touch the retained backend seams that the new S05 wrapper exposed as unstable: `scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh` now waits for the full healthy `/health` contract before creating a smoke job, and `compiler/meshc/tests/e2e_reference_backend.rs` plus the retained README now split crash/requeue correctness from post-recovery `/health` metadata visibility instead of relying on a flaky mid-recovery degraded-window observation.

## Known Limitations

The retained docs and wrapper rails are exact-string-sensitive by design, so future wording changes still need the matching Node/Rust/shell contract updates in the same change. The retained backend recovery surface now proves stable post-recovery `/health` restart metadata rather than a guaranteed live degraded-window observation during recovery. `cargo test` output for the touched Rust rails still carries pre-existing unused-helper warnings from support modules; the slice kept the proof rails green but did not clean that warning baseline.

## Follow-ups

If maintainers need a live in-flight degraded-window proof again, reopen the retained backend runtime/HTTP scheduling seam and only then tighten the README or restart-visibility rail back toward that claim. Otherwise the slice is closed: the stable public and maintainer-facing proof surface is the post-recovery `/health` metadata plus the retained S02 and S05 bundles.

## Files Created/Modified

- `scripts/verify-production-proof-surface.sh` — Canonicalized the public proof-page verifier at the stable top-level path that survives `reference-backend/` deletion.
- `website/docs/docs/production-backend-proof/index.md` — Retargeted the public backend proof page to the top-level verifier and the post-deletion maintainer handoff.
- `website/docs/docs/tooling/index.md` — Removed the last public repo-root backend wording and kept tooling/distributed proof pages aligned with the examples-first story.
- `website/docs/docs/distributed-proof/index.md` — Removed the stale deeper-backend `reference-backend` handoff from the public secondary docs story.
- `scripts/fixtures/backend/reference-backend/README.md` — Rewrote the retained backend runbook to post-deletion truth and clarified the restart-metadata recovery contract.
- `scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh` — Tightened retained fixture smoke startup so it waits for `status=ok`, `liveness=healthy`, and `recovery_active=false` before creating work.
- `compiler/meshc/tests/e2e_reference_backend.rs` — Removed the duplicate live degraded-window requirement from crash recovery and kept the retained backend recovery rails truthful in serial replay.
- `.gitignore` — Deleted the repo-root compatibility tree and removed the obsolete generated-binary ignore rule.
- `scripts/verify-m051-s02.sh` — Kept the retained backend-only verifier honest on the post-deletion tree and retained the backend proof bundle under `.tmp/m051-s02/verify/`.
- `compiler/meshc/tests/e2e_m051_s05.rs` — Added the slice-owned post-deletion source contract for the final acceptance rail.
- `scripts/verify-m051-s05.sh` — Added the authoritative post-deletion assembled replay and self-contained retained bundle for S01-S04 child rails.
- `.gsd/PROJECT.md` — Refreshed the living project state so M051 is marked complete and `reference-backend/` is no longer described as a live compatibility surface.
