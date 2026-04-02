---
estimated_steps: 4
estimated_files: 4
skills_used:
  - vscode-extension-expert
  - test
---

# T01: Make the public README/tooling/VS Code touchpoints truthful and pin them with an S05 docs contract test

**Slice:** S05 — Assembled contract proof and minimal public touchpoints
**Milestone:** M048

## Description

This task closes the public-truth gap that remains after S02/S03/S04. The product surfaces already exist; the risk is that first-contact docs and the VS Code README still omit installer-backed updates, optional override entrypoints, nested-source publish truth, or the bounded editor proof surface. Update only the minimal stale public touchpoints and add one dedicated S05 contract test so later rewording fails closed.

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

## Verification

- `node --test scripts/tests/verify-m048-s05-contract.test.mjs`
- `npm --prefix website run build`

## Inputs

- `README.md` — current public root README that still omits the new update/entrypoint/verifier truth.
- `website/docs/docs/tooling/index.md` — main public tooling page that needs the update, entrypoint, publish, and editor truth adjustments.
- `tools/editors/vscode-mesh/README.md` — VS Code-specific public touchpoint with the stale cross-file definition overclaim.
- `tools/editors/vscode-mesh/src/test/suite/extension.test.ts` — authoritative VS Code smoke surface for hover, same-file definition, and the override-entry fixture.
- `compiler/meshc/tests/e2e_m048_s01.rs` — retained override-entry build/test rail that defines the R112 public truth.
- `compiler/meshc/tests/tooling_e2e.rs` — retained help/discovery coverage for public `meshc` CLI touchpoints.
- `compiler/meshpkg/tests/update_cli.rs` — retained `meshpkg update` help and JSON-guard truth for R113.
- `scripts/tests/verify-m036-s02-contract.test.mjs` — retained editor-contract rail to stay consistent with bounded host claims.
- `scripts/tests/verify-m048-s04-skill-contract.test.mjs` — retained grammar/skill truth rail that should remain consistent with public wording.

## Expected Output

- `README.md` — public root README updated with truthful update, entrypoint, and verifier notes.
- `website/docs/docs/tooling/index.md` — tooling docs updated with update-command, override-entry, publish, and editor truth.
- `tools/editors/vscode-mesh/README.md` — VS Code README narrowed to the actual proof surface.
- `scripts/tests/verify-m048-s05-contract.test.mjs` — fail-closed public-touchpoint drift rail for S05.
