# S05: Assembled contract proof and minimal public touchpoints

**Goal:** Close M048 with one retained verifier and a minimal public-truth pass so override-entry, self-update, grammar parity, and editor/package touchpoints all match the shipped contract.
**Demo:** After this: After this: one retained verifier proves the override-entry project, self-update commands, grammar parity, and refreshed skill contract together, and the minimal public touchpoints stop lying about these surfaces.

## Tasks
- [x] **T01: Updated public Mesh docs for update and override-entry truth, and added the S05 fail-closed docs contract test.** — This task closes the public-truth gap that remains after S02/S03/S04. The product surfaces already exist; the risk is that first-contact docs and the VS Code README still omit installer-backed updates, optional override entrypoints, nested-source publish truth, or the bounded editor proof surface. Update only the minimal stale public touchpoints and add one dedicated S05 contract test so later rewording fails closed.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `README.md`, `website/docs/docs/tooling/index.md`, and `tools/editors/vscode-mesh/README.md` | Fail the contract test with the exact file + missing or extra claim; do not relax wording until it matches real proof surfaces. | N/A for local file reads. | Treat reintroduced stale claims or missing update / entrypoint / grammar markers as contract drift. |
| `tools/editors/vscode-mesh/src/test/suite/extension.test.ts` plus retained S02/S03/S04 rails | Keep docs scoped to hover, same-file definition, override-entry fixture, update commands, and shared grammar truth actually proven today. | N/A for repo reads. | Reject wording that implies cross-file definition on the override-entry path or unsupported editor/update surfaces. |
| `node --test` and `npm --prefix website run build` | Stop the task on a failing assertion or docs build error. | Fail closed rather than merging partial doc edits. | Treat broken markdown, missing verifier paths, or stale exact-string markers as proof failure. |

## Load Profile

- **Shared resources**: repo markdown surfaces, one Node contract test, and one VitePress build.
- **Per-operation cost**: fixed string assertions plus one docs build; trivial compared with the retained runtime rails.
- **10x breakpoint**: exact-marker drift and markdown/build errors fail before performance matters, so optimize for precise minimal wording instead of broader rewrites.

## Negative Tests

- **Malformed inputs**: missing `meshc update`, `meshpkg update`, `[package].entrypoint`, `lib/start.mpl`, `@cluster`, or `bash scripts/verify-m048-s05.sh` markers in the touched docs.
- **Error paths**: reintroducing `jump to definitions across files`, overstating override-entry definition support, or forgetting the nested-source publish note should fail the contract test.
- **Boundary conditions**: keep `main.mpl` as the default while documenting `[package].entrypoint` as optional, and keep VS Code claims limited to the proof surface actually exercised today.

## Steps

1. Update `README.md` so install/quick-start mention `meshc update` / `meshpkg update`, keep `main.mpl` as the default executable entrypoint, explain optional `[package].entrypoint = "lib/start.mpl"`, and point readers at `bash scripts/verify-m048-s05.sh`.
2. Update `website/docs/docs/tooling/index.md` with a short toolchain-update subsection, an override-entry `mesh.toml` example, a truthful `meshpkg publish` note about preserving nested project-root-relative `.mpl` paths while excluding hidden/test-only files, a manifest-first editor/grammar note, and the new assembled verifier entrypoint.
3. Narrow `tools/editors/vscode-mesh/README.md` to the proof that actually ships: remove the stale cross-file definition overclaim, mention `@cluster` / `@cluster(N)` plus both interpolation forms, and document the manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`.
4. Add `scripts/tests/verify-m048-s05-contract.test.mjs` so these three public touchpoints must include the new truth markers and the VS Code README must omit `jump to definitions across files`.

## Must-Haves

- [ ] `README.md` now teaches installer-backed update commands, optional `[package].entrypoint`, and the S05 assembled verifier while keeping `main.mpl` as the default.
- [ ] `website/docs/docs/tooling/index.md` now teaches canonical installer-backed updates, override-entry manifests, nested-source publish truth, manifest-first editor proof, and the S05 verifier.
- [ ] `tools/editors/vscode-mesh/README.md` now matches the actual proof surface and omits the stale `jump to definitions across files` claim.
- [ ] `scripts/tests/verify-m048-s05-contract.test.mjs` fails closed on missing truth markers or reintroduced stale wording.
  - Estimate: 2h
  - Files: README.md, website/docs/docs/tooling/index.md, tools/editors/vscode-mesh/README.md, scripts/tests/verify-m048-s05-contract.test.mjs
  - Verify: - `node --test scripts/tests/verify-m048-s05-contract.test.mjs`
- `npm --prefix website run build`
- [ ] **T02: Assemble the retained S01-S04 rails into `scripts/verify-m048-s05.sh` with diagnosable proof artifacts** — Once the public text is truthful, the slice still needs one named closeout entrypoint that proves the assembled contract without rediscovering product behavior. This task adds the retained S05 verifier as an orchestration layer over the existing S01-S04 rails, with M047-style phase bookkeeping and artifact retention so the first failing seam is inspectable.

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
  - Estimate: 2h
  - Files: scripts/verify-m048-s05.sh
  - Verify: - `bash scripts/verify-m048-s05.sh`
- `test "$(cat .tmp/m048-s05/verify/status.txt)" = "ok" && test "$(cat .tmp/m048-s05/verify/current-phase.txt)" = "complete"`
