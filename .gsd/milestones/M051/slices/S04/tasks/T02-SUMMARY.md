---
id: T02
parent: S04
milestone: M051
provides: []
requires: []
affects: []
key_files: ["website/docs/docs/production-backend-proof/index.md", "website/docs/docs/distributed/index.md", "website/docs/docs/distributed-proof/index.md", "website/docs/docs/web/index.md", "website/docs/docs/databases/index.md", "website/docs/docs/testing/index.md", "website/docs/docs/concurrency/index.md", "reference-backend/scripts/verify-production-proof-surface.sh", "scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs", ".gsd/DECISIONS.md", ".gsd/KNOWLEDGE.md", ".gsd/milestones/M051/slices/S04/tasks/T02-SUMMARY.md"]
key_decisions: ["Keep `/docs/production-backend-proof/` as the compact public-secondary handoff and route deeper maintainer work to `mesher/README.md`, `bash scripts/verify-m051-s01.sh`, and `bash scripts/verify-m051-s02.sh` instead of public `reference-backend/README.md` teaching.", "Make both the source contract and the compatibility proof-page verifier fail closed on stale repo-root backend handoffs and leaked retained-fixture paths."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Ran the task-owned source contract and compatibility verifier after the docs and verifier rewrites. `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` passed with all four subtests green against the live tree, proving the new Mesher plus retained-backend handoff and its negative cases. `bash reference-backend/scripts/verify-production-proof-surface.sh` passed and confirmed the proof-page role, sidebar/footer placement, guide handoffs, and stale-surface denials directly from source."
completed_at: 2026-04-04T18:47:03.639Z
blocker_discovered: false
---

# T02: Retargeted the public-secondary backend docs and proof verifiers to Mesher plus named retained backend replays.

> Retargeted the public-secondary backend docs and proof verifiers to Mesher plus named retained backend replays.

## What Happened
---
id: T02
parent: S04
milestone: M051
key_files:
  - website/docs/docs/production-backend-proof/index.md
  - website/docs/docs/distributed/index.md
  - website/docs/docs/distributed-proof/index.md
  - website/docs/docs/web/index.md
  - website/docs/docs/databases/index.md
  - website/docs/docs/testing/index.md
  - website/docs/docs/concurrency/index.md
  - reference-backend/scripts/verify-production-proof-surface.sh
  - scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs
  - .gsd/DECISIONS.md
  - .gsd/KNOWLEDGE.md
  - .gsd/milestones/M051/slices/S04/tasks/T02-SUMMARY.md
key_decisions:
  - Keep `/docs/production-backend-proof/` as the compact public-secondary handoff and route deeper maintainer work to `mesher/README.md`, `bash scripts/verify-m051-s01.sh`, and `bash scripts/verify-m051-s02.sh` instead of public `reference-backend/README.md` teaching.
  - Make both the source contract and the compatibility proof-page verifier fail closed on stale repo-root backend handoffs and leaked retained-fixture paths.
duration: ""
verification_result: passed
completed_at: 2026-04-04T18:47:03.641Z
blocker_discovered: false
---

# T02: Retargeted the public-secondary backend docs and proof verifiers to Mesher plus named retained backend replays.

**Retargeted the public-secondary backend docs and proof verifiers to Mesher plus named retained backend replays.**

## What Happened

Rewrote `website/docs/docs/production-backend-proof/index.md` so the page stays public-secondary and footer-opted-out but now explains the new split explicitly: public readers stay on scaffold/examples-first surfaces, repo maintainers go deeper through `mesher/README.md` plus `bash scripts/verify-m051-s01.sh`, and the retained backend-only proof stays behind `bash scripts/verify-m051-s02.sh` instead of a public repo-root runbook handoff. Updated `distributed`, `distributed-proof`, `web`, `databases`, `testing`, and `concurrency` so they route backend-specific readers through Production Backend Proof first and only then mention the maintainer-facing Mesher / retained-proof surfaces. Rewrote `reference-backend/scripts/verify-production-proof-surface.sh` to keep the compatibility file path alive while checking the new proof-page role, sidebar/footer placement, Mesher handoff markers, retained-proof command markers, leaked fixture-path denial, and stale repo-root runbook denial. Rewrote `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` so the source contract fails closed on the new Mesher/retained-proof story rather than the old `reference-backend/README.md` handoff, then recorded the public contract decision in `.gsd/DECISIONS.md` and the repeated-link order-assertion gotcha in `.gsd/KNOWLEDGE.md`.

## Verification

Ran the task-owned source contract and compatibility verifier after the docs and verifier rewrites. `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` passed with all four subtests green against the live tree, proving the new Mesher plus retained-backend handoff and its negative cases. `bash reference-backend/scripts/verify-production-proof-surface.sh` passed and confirmed the proof-page role, sidebar/footer placement, guide handoffs, and stale-surface denials directly from source.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` | 0 | ✅ pass | 233ms |
| 2 | `bash reference-backend/scripts/verify-production-proof-surface.sh` | 0 | ✅ pass | 1578ms |


## Deviations

Also replaced the public Testing guide's stale `reference-backend` example commands with generic `my-app` examples so the page no longer teaches the compatibility package directly.

## Known Issues

None.

## Files Created/Modified

- `website/docs/docs/production-backend-proof/index.md`
- `website/docs/docs/distributed/index.md`
- `website/docs/docs/distributed-proof/index.md`
- `website/docs/docs/web/index.md`
- `website/docs/docs/databases/index.md`
- `website/docs/docs/testing/index.md`
- `website/docs/docs/concurrency/index.md`
- `reference-backend/scripts/verify-production-proof-surface.sh`
- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
- `.gsd/DECISIONS.md`
- `.gsd/KNOWLEDGE.md`
- `.gsd/milestones/M051/slices/S04/tasks/T02-SUMMARY.md`


## Deviations
Also replaced the public Testing guide's stale `reference-backend` example commands with generic `my-app` examples so the page no longer teaches the compatibility package directly.

## Known Issues
None.
