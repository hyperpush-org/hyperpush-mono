---
estimated_steps: 5
estimated_files: 1
skills_used:
  - test
  - neovim
---

# T02: Assemble the retained S01-S04 rails into `scripts/verify-m048-s05.sh` with diagnosable proof artifacts

**Slice:** S05 — Assembled contract proof and minimal public touchpoints
**Milestone:** M048

## Description

Once the public text is truthful, the slice still needs one named closeout entrypoint that proves the assembled contract without rediscovering product behavior. This task adds the retained S05 verifier as an orchestration layer over the existing S01-S04 rails, with M047-style phase bookkeeping and artifact retention so the first failing seam is inspectable.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `scripts/tests/verify-m048-s05-contract.test.mjs` | Stop before the long replay and point directly at the drifting public touchpoint. | Fail the first phase and keep the docs-contract log. | Reject missing markers or the stale VS Code overclaim as contract drift. |
| Retained `cargo test` rails for S01 and S03 | Stop on the first red phase, write the failing command log, and preserve any retained timestamped artifact directory for inspection. | Kill the child process, mark the phase failed, and record the timeout in the phase log. | Reject zero-test filters, missing pass markers, or absent artifact snapshots as verifier drift. |
| `scripts/verify-m036-s02.sh` plus `nvim` and `npm --prefix tools/editors/vscode-mesh run test:smoke` | Keep the truthful `NEOVIM_BIN="${NEOVIM_BIN:-nvim}"` entrypoint, build ordering, and upstream artifact hints instead of assuming a vendor binary or already-built `meshc`. | Fail the named phase with its upstream artifact directory and do not continue into later rails. | Reject missing `target/debug/meshc`, missing `test:smoke`, or missing Neovim support files as contract drift. |
| `npm --prefix website run build` and retained bundle-copy helpers | Stop final proof on docs build or bundle-shape failure and leave the failing log/manifests in `.tmp/m048-s05/verify`. | Fail the docs-build or bundle-shape phase instead of hanging. | Reject missing `status.txt`, `phase-report.txt`, retained bundle pointers, or malformed copied-artifact manifests. |

## Load Profile

- **Shared resources**: `target/debug` binaries, `.tmp/m048-s05/verify`, fixed `.tmp/m036-s02/*` / `.tmp/m036-s03/*` directories, timestamped `.tmp/m048-s01/*` / `.tmp/m048-s03/*` buckets, and website build output.
- **Per-operation cost**: one end-to-end verifier replay across the retained S01/S02/S03/S04 rails plus docs build and artifact copying.
- **10x breakpoint**: build/runtime timeouts and artifact churn fail before CPU saturation, so the script must keep bounded waits, named phases, and explicit copy rules.

## Negative Tests

- **Malformed inputs**: missing contract test file, unsupported verifier phase, absent `test:smoke` script, or absent Neovim binary.
- **Error paths**: a cargo phase fails, VS Code smoke fails before suite start, docs build fails, or retained artifact copies are missing required files/pointers.
- **Boundary conditions**: run at least one cargo phase before VS Code smoke so `target/debug/meshc` exists; use direct copies for fixed M036 directories and snapshot/copy logic only for timestamped M048 buckets.

## Steps

1. Use `scripts/verify-m047-s05.sh` and `scripts/verify-m047-s06.sh` as the shell-pattern references for a new `scripts/verify-m048-s05.sh` that owns `.tmp/m048-s05/verify`, `status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, and `latest-proof-bundle.txt`.
2. Make `node --test scripts/tests/verify-m048-s05-contract.test.mjs` the first phase, then replay the retained rails in a truthful order: S01 entrypoint, S02 LSP/Neovim and VS Code smoke, S02 publish coverage, S03 update rails, S04 grammar/Neovim contract/skill rails, and `npm --prefix website run build`.
3. Keep the watchouts explicit in the script: use `NEOVIM_BIN="${NEOVIM_BIN:-nvim}"` instead of the missing repo-vendor path, run a cargo phase before VS Code smoke so `target/debug/meshc` exists, and fail closed when any named test filter, helper script, or retained artifact is missing.
4. Retain proof artifacts without lying about shape: copy fixed M036 directories directly, snapshot/copy fresh timestamped `.tmp/m048-s01/*` and `.tmp/m048-s03/*` buckets into the retained bundle, and write a stable latest-bundle pointer for future milestone validation.
5. Run `bash scripts/verify-m048-s05.sh` end to end and keep the pass marker plus retained bundle layout stable for future closeout replay.

## Must-Haves

- [ ] `scripts/verify-m048-s05.sh` uses named phases plus `.tmp/m048-s05/verify` bookkeeping files modeled after the retained closeout wrappers.
- [ ] The verifier fails fast on `scripts/tests/verify-m048-s05-contract.test.mjs` before starting the long replay.
- [ ] The verifier replays the retained S01/S02/S03/S04 rails with truthful `NEOVIM_BIN` handling, cargo-before-VS-Code ordering, and publish/update/grammar coverage intact.
- [ ] The retained bundle captures both fixed M036 directories and fresh timestamped M048 artifacts, with a stable pointer and fail-closed shape checks.

## Verification

- `bash scripts/verify-m048-s05.sh`
- `test "$(cat .tmp/m048-s05/verify/status.txt)" = "ok" && test "$(cat .tmp/m048-s05/verify/current-phase.txt)" = "complete"`

## Observability Impact

- Signals added/changed: `.tmp/m048-s05/verify/status.txt`, `.tmp/m048-s05/verify/current-phase.txt`, `.tmp/m048-s05/verify/phase-report.txt`, `.tmp/m048-s05/verify/full-contract.log`, and `.tmp/m048-s05/verify/latest-proof-bundle.txt`.
- How a future agent inspects this: rerun `bash scripts/verify-m048-s05.sh`, inspect the first red phase in `phase-report.txt`, then follow the retained bundle pointer into copied M036/M048 artifacts.
- Failure state exposed: first failing phase, timed-out command, missing retained-artifact file, or malformed bundle shape is visible without rerunning under ad hoc instrumentation.

## Inputs

- `scripts/verify-m047-s05.sh` — retained closeout-shell pattern for phase bookkeeping and artifact retention.
- `scripts/verify-m047-s06.sh` — retained wrapper pattern for delegated verification and retained-bundle shape checks.
- `scripts/verify-m036-s02.sh` — truthful Neovim/LSP replay entrypoint that must use `NEOVIM_BIN` rather than the missing vendor binary.
- `scripts/tests/verify-m048-s05-contract.test.mjs` — fail-fast docs contract phase to run before the long replay.
- `compiler/meshc/tests/e2e_m048_s01.rs` — authoritative override-entry replay for R112.
- `compiler/meshc/tests/e2e_m048_s03.rs` — retained staged/installed toolchain-update replay for R113.
- `compiler/mesh-pkg/tests/toolchain_update.rs` — focused updater seam rail that stays in the assembled verifier.
- `compiler/meshpkg/tests/update_cli.rs` — retained `meshpkg update` CLI/help guard for R113.
- `tools/editors/vscode-mesh/src/test/runTest.ts` — VS Code smoke harness with the `target/debug/meshc` precondition.
- `scripts/tests/verify-m036-s02-contract.test.mjs` — retained editor-contract rail from S02/S04.
- `scripts/tests/verify-m048-s04-skill-contract.test.mjs` — retained grammar/skill drift rail for R114.

## Expected Output

- `scripts/verify-m048-s05.sh` — assembled named-phase verifier that replays the retained rails and writes a diagnosable retained bundle.
