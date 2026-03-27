# S11: First-green archive and final assembly closeout

**Goal:** Retire the remaining hosted release-lane blocker on the approved rollout refs, then capture the one-shot `first-green` archive and finish a fresh full S05 assembly replay for milestone revalidation.
**Demo:** After this: `VERIFY_M034_S05_STOP_AFTER=remote-evidence bash scripts/verify-m034-s05.sh` is green, `.tmp/m034-s06/evidence/first-green/` is captured exactly once, and the full `bash scripts/verify-m034-s05.sh` assembly replay finishes green for milestone revalidation.

## Tasks
- [x] **T01: Proved the Windows release-smoke workflow patch is already aligned locally and captured fresh T01 verifier logs for the rollout handoff.** — Why: S11 cannot get a truthful hosted green `release.yml` lane until the only remaining repo-local drift — the Windows smoke-toolchain steps in `.github/workflows/release.yml` and the matching assertions in `scripts/verify-m034-s02-workflows.sh` — is finished and re-proven locally.

Files: `.github/workflows/release.yml`, `scripts/verify-m034-s02-workflows.sh`, `scripts/verify-m034-s03.ps1`, `scripts/tests/verify-m034-s03-last-exitcode.ps1`.

Do:
- Reproduce the current local release-workflow diff and keep the workflow and contract verifier aligned on the Windows smoke-toolchain steps.
- Touch `scripts/verify-m034-s03.ps1` only if replayed diagnostics show a deeper Windows staged-smoke contract gap than the current workflow patch.
- Re-run the local workflow-contract and PowerShell helper proofs, and preserve fresh T01 logs under `.tmp/m034-s11/t01/` so T02 can distinguish local drift from rollout-only blockers.

Done when: the intended release-workflow patch is present locally, `bash scripts/verify-m034-s02-workflows.sh` is green, and the remaining blocker surface is isolated to hosted rollout rather than stale repo-local workflow drift.
  - Estimate: 1h
  - Files: .github/workflows/release.yml, scripts/verify-m034-s02-workflows.sh, scripts/verify-m034-s03.ps1, scripts/tests/verify-m034-s03-last-exitcode.ps1, .tmp/m034-s05/verify/remote-runs.json
  - Verify: bash scripts/verify-m034-s02-workflows.sh
pwsh -NoProfile -File scripts/tests/verify-m034-s03-last-exitcode.ps1
bash -n scripts/verify-m034-s03.sh
- [x] **T02: Roll the approved release fixes onto the hosted refs and capture `first-green` exactly once** — Why: R007 and the slice demo depend on fresh hosted proof, not stale local passes. This task rolls the repaired release lane onto the expected refs, refreshes `remote-evidence`, and spends the one-shot `first-green` label only after the canonical stop-after gate is actually green.

Files: `.github/workflows/release.yml`, `scripts/verify-m034-s05.sh`, `scripts/verify-m034-s06-remote-evidence.sh`, `.tmp/m034-s05/verify/remote-runs.json`, `.tmp/m034-s06/evidence/first-green/manifest.json`, `.tmp/m034-s09/rollout/target-sha.txt`.

Do:
- Inspect local/remote ref drift and prepare the exact outward-action summary for the user; get explicit confirmation before any push, tag update, or hosted workflow rerun.
- After approval, roll forward only the refs that S05 actually gates on; keep the split binary vs extension tag model intact unless extension files changed.
- Replay `VERIFY_M034_S05_STOP_AFTER=remote-evidence bash scripts/verify-m034-s05.sh` until `.tmp/m034-s05/verify/remote-runs.json` shows every workflow at `status: ok` on its expected ref, then run `bash scripts/verify-m034-s06-remote-evidence.sh first-green` exactly once.
- Preserve refreshed hosted logs and a local summary under `.tmp/m034-s11/t02/` so T03 can trust the archive without re-reading GitHub manually.

Done when: `first-green` exists with `s05Status: ok`, `currentPhase: stopped-after-remote-evidence`, no later S05 phases ran, and every remote-run summary is green on the expected head SHA/ref.
  - Estimate: 1h 30m
  - Files: .github/workflows/release.yml, scripts/verify-m034-s05.sh, scripts/verify-m034-s06-remote-evidence.sh, .tmp/m034-s05/verify/remote-runs.json, .tmp/m034-s06/evidence/first-green/manifest.json, .tmp/m034-s09/rollout/target-sha.txt, .tmp/m034-s09/rollout/workflow-status.json
  - Verify: VERIFY_M034_S05_STOP_AFTER=remote-evidence bash scripts/verify-m034-s05.sh
bash scripts/verify-m034-s06-remote-evidence.sh first-green
- [x] **T03: Confirmed T03 is blocked because `release.yml` still fails on the approved `v0.1.0` rollout ref, leaving `first-green` absent and preventing the final S05 replay.** — Why: `first-green` only proves hosted freshness through `remote-evidence`. Milestone revalidation still needs the full public-surface and live publish/install replay to finish green from a fresh verify root.

Files: `scripts/verify-m034-s05.sh`, `.env`, `.tmp/m034-s05/verify/status.txt`, `.tmp/m034-s05/verify/current-phase.txt`, `.tmp/m034-s05/verify/phase-report.txt`, `.tmp/m034-s06/evidence/first-green/manifest.json`.

Do:
- Confirm the `first-green` manifest exists and still matches the expected binary and extension refs before the full replay.
- Source `.env` in-process and run the full blocking `bash scripts/verify-m034-s05.sh` path; use the fresh phase artifacts rather than async wrappers so the evidence stays authoritative.
- Assert `.tmp/m034-s05/verify/status.txt = ok`, `.tmp/m034-s05/verify/current-phase.txt = complete`, and passed `remote-evidence`, `public-http`, and `s01-live-proof` entries in `.tmp/m034-s05/verify/phase-report.txt`.
- Write a compact closeout summary under `.tmp/m034-s11/t03/` linking the `first-green` manifest to the final S05 verify root and stop red if any phase regresses.

Done when: the full S05 replay finishes green from a clean verify root and the final summary ties the milestone-closeout proof back to the one-shot `first-green` archive.
  - Estimate: 45m
  - Files: scripts/verify-m034-s05.sh, .env, .tmp/m034-s05/verify/status.txt, .tmp/m034-s05/verify/current-phase.txt, .tmp/m034-s05/verify/phase-report.txt, .tmp/m034-s06/evidence/first-green/manifest.json
  - Verify: set -euo pipefail; test -f .env; set -a; source .env; set +a; bash scripts/verify-m034-s05.sh
grep -Fxq 'ok' .tmp/m034-s05/verify/status.txt
grep -Fxq 'complete' .tmp/m034-s05/verify/current-phase.txt
grep -Fxq $'remote-evidence	passed' .tmp/m034-s05/verify/phase-report.txt
grep -Fxq $'public-http	passed' .tmp/m034-s05/verify/phase-report.txt
grep -Fxq $'s01-live-proof	passed' .tmp/m034-s05/verify/phase-report.txt
  - Blocker: `release.yml` run `23669185030` is still failing on `v0.1.0` at SHA `1e83ea930fdfd346b9e56659dc50d2f759ec5da2`. The blocking hosted job is `Verify release assets (x86_64-pc-windows-msvc)`, which crashes during `installed meshc.exe build installer smoke fixture` with access-violation exit code `-1073741819`. `.tmp/m034-s06/evidence/first-green/manifest.json` does not exist, so the final S05 assembly replay cannot be claimed truthfully.
