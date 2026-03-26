---
id: T02
parent: S05
milestone: M033
key_files:
  - scripts/verify-m033-s05.sh
  - website/docs/docs/databases/index.md
  - scripts/verify-m033-s02.sh
  - scripts/verify-m033-s03.sh
  - scripts/verify-m033-s04.sh
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Made `scripts/verify-m033-s05.sh` the canonical assembled acceptance command, with named phase logs under `.tmp/m033-s05/verify/` and a strict exact-string docs-truth sweep before the live-Postgres replay.
  - Updated the duplicated verifier `fn_block(...)` helpers in `scripts/verify-m033-s02.sh`, `scripts/verify-m033-s03.sh`, and `scripts/verify-m033-s04.sh` to stop at private `fn` declarations as well as `pub fn` so proof sweeps inspect only the intended public function bodies.
  - Realigned S02’s raw-boundary ownership by dropping `get_event_alert_rules` and `get_threshold_rules` from its builder-owned keep-list, because they are now deliberate S03 raw keep-sites and are documented that way in the public contract.
duration: ""
verification_result: passed
completed_at: 2026-03-26T04:36:20.938Z
blocker_discovered: false
---

# T02: Added the canonical S05 acceptance wrapper, locked the docs truth surface, and repaired stale verifier ownership so the full replay passes.

**Added the canonical S05 acceptance wrapper, locked the docs truth surface, and repaired stale verifier ownership so the full replay passes.**

## What Happened

Added `scripts/verify-m033-s05.sh` as the canonical assembled acceptance command for this slice. The wrapper now runs `npm --prefix website run build`, then a Python exact-string truth sweep over `website/docs/docs/databases/index.md`, then `bash scripts/verify-m033-s02.sh`, `bash scripts/verify-m033-s03.sh`, and `bash scripts/verify-m033-s04.sh` strictly serially while preserving named phase logs under `.tmp/m033-s05/verify/` so the first failing phase is obvious. I tightened `website/docs/docs/databases/index.md` to include the canonical `bash scripts/verify-m033-s05.sh` command both in the proof-command list and the proof/failure map so the public page matches the verifier’s enforced contract.

While bringing the wrapper up against local reality, the existing S02/S03/S04 verifier Python helper that extracts public Mesh functions was too coarse: it stopped only at the next `pub fn`, which caused later private helpers to be swallowed into earlier public-function checks. I fixed that parser boundary in all three scripts and removed the stale S02 ownership assertions for `get_event_alert_rules` and `get_threshold_rules`, which are now deliberate S03 raw keep-sites. After those verifier repairs, the standalone docs build and the full `bash scripts/verify-m033-s05.sh` replay both passed. During wrap-up I also reproduced a false VitePress `ERR_MODULE_NOT_FOUND` failure by running the standalone docs build and the S05 wrapper concurrently; I reran the required commands serially, confirmed the acceptance path is green, and documented that concurrency gotcha in `.gsd/KNOWLEDGE.md`.

## Verification

Verified `npm --prefix website run build` passes in isolation. Verified `bash scripts/verify-m033-s05.sh` passes end-to-end and serially replays the docs build, docs-truth sweep, and the S02/S03/S04 live-Postgres verifiers. Verified the observability surface by checking that `.tmp/m033-s05/verify/01-docs-build.log` through `.tmp/m033-s05/verify/05-verify-m033-s04.log` exist after the successful run.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `npm --prefix website run build` | 0 | ✅ pass | 50687ms |
| 2 | `bash scripts/verify-m033-s05.sh` | 0 | ✅ pass | 494426ms |
| 3 | `test -f .tmp/m033-s05/verify/01-docs-build.log && test -f .tmp/m033-s05/verify/02-docs-truth.log && test -f .tmp/m033-s05/verify/03-verify-m033-s02.log && test -f .tmp/m033-s05/verify/04-verify-m033-s03.log && test -f .tmp/m033-s05/verify/05-verify-m033-s04.log && ls -1 .tmp/m033-s05/verify` | 0 | ✅ pass | 57ms |


## Deviations

Expanded the task slightly to patch stale verifier parsing and ownership checks in `scripts/verify-m033-s02.sh`, `scripts/verify-m033-s03.sh`, and `scripts/verify-m033-s04.sh` so the new S05 wrapper could replay the real current proof surface. No slice replan was needed.

## Known Issues

Concurrent `npm --prefix website run build` and `bash scripts/verify-m033-s05.sh` runs can race VitePress's shared `website/docs/.vitepress/.temp` output and produce false `ERR_MODULE_NOT_FOUND` failures. The required acceptance path is serial, it passes, and the gotcha is documented in `.gsd/KNOWLEDGE.md`.

## Files Created/Modified

- `scripts/verify-m033-s05.sh`
- `website/docs/docs/databases/index.md`
- `scripts/verify-m033-s02.sh`
- `scripts/verify-m033-s03.sh`
- `scripts/verify-m033-s04.sh`
- `.gsd/KNOWLEDGE.md`
