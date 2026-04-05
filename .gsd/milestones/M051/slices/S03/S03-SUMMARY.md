---
id: S03
parent: M051
milestone: M051
provides:
  - A canonical retained backend fixture proof root for bounded tooling, formatter, LSP, and editor-host rails.
  - Public editor-host README/docs contract guards that keep the internal retained fixture out of the onboarding path while preserving truthful proof wording.
  - One assembled retained verification bundle under `.tmp/m051-s03/verify/` that downstream slices can reuse when checking the tooling/editor cutover.
requires:
  - slice: S02
    provides: The retained backend-only fixture under `scripts/fixtures/backend/reference-backend/` plus the S02 helper/verifier pattern that preserved compatibility while consumers were cut over.
affects:
  - S04
  - S05
key_files:
  - compiler/meshc/tests/support/m051_reference_backend.rs
  - compiler/meshc/tests/e2e_m051_s03.rs
  - compiler/meshc/tests/e2e_lsp.rs
  - compiler/meshc/tests/tooling_e2e.rs
  - compiler/meshc/tests/e2e_fmt.rs
  - compiler/mesh-lsp/src/analysis.rs
  - compiler/mesh-fmt/src/lib.rs
  - tools/editors/vscode-mesh/src/test/suite/extension.test.ts
  - tools/editors/neovim-mesh/tests/smoke.lua
  - scripts/fixtures/m036-s01-syntax-corpus.json
  - tools/editors/vscode-mesh/README.md
  - tools/editors/neovim-mesh/README.md
  - scripts/tests/verify-m036-s03-contract.test.mjs
  - scripts/verify-m051-s03.sh
  - .gsd/PROJECT.md
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
key_decisions:
  - Use the retained backend fixture at `scripts/fixtures/backend/reference-backend/` as the bounded tooling/editor/LSP/formatter proof root instead of repo-root `reference-backend/` or full Mesher.
  - Keep formatter proof bounded to canonical retained files/subtrees that are actually clean today; do not overclaim the whole retained fixture root while `tests/fixture.test.mpl` remains known-red.
  - Keep the public VS Code and Neovim READMEs generic about a backend-shaped proof surface and fail closed on leaked repo-root or retained-fixture paths (recorded as D385).
  - Make `scripts/verify-m051-s03.sh` the authoritative assembled S03 replay and copy delegated M036 evidence into its own retained bundle instead of mutating historical `.tmp` trees in place.
patterns_established:
  - Centralize retained backend fixture paths and helper behavior in `compiler/meshc/tests/support/m051_reference_backend.rs` so Rust tooling rails stop hardcoding stale proof roots.
  - Use mutation-based README/docs contract tests with explicit include/exclude assertions to keep public wording honest without hand-auditing prose.
  - Treat assembled verifier output as an observable product surface: phase-labeled replay, `status.txt` / `current-phase.txt` / `phase-report.txt`, and a retained bundle pointer are part of the contract.
  - Keep editor-host proof bounded to manifest-first root detection, clean diagnostics, hover, same-file definition in backend-shaped code, and explicit override-entry/single-file cases rather than broadening claims implicitly.
observability_surfaces:
  - `.tmp/m051-s03/verify/status.txt`
  - `.tmp/m051-s03/verify/current-phase.txt`
  - `.tmp/m051-s03/verify/phase-report.txt`
  - `.tmp/m051-s03/verify/full-contract.log`
  - `.tmp/m051-s03/verify/latest-proof-bundle.txt`
  - `.tmp/m036-s03/vscode-smoke/smoke.log`
  - `.tmp/m036-s02/syntax/neovim-smoke.log`
  - `.tmp/m036-s02/lsp/neovim-smoke.log`
drill_down_paths:
  - .gsd/milestones/M051/slices/S03/tasks/T01-SUMMARY.md
  - .gsd/milestones/M051/slices/S03/tasks/T02-SUMMARY.md
  - .gsd/milestones/M051/slices/S03/tasks/T03-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-04T16:49:39.421Z
blocker_discovered: false
---

# S03: Migrate tooling and editor rails to a bounded backend fixture

**Retargeted the bounded tooling, formatter, LSP, and editor-host proof rails from repo-root `reference-backend/` to the retained backend fixture, kept public editor docs generic about that proof surface, and closed the cutover with one retained assembled verifier.**

## What Happened

S03 finished the tooling/editor side of the `reference-backend/` retirement by moving every bounded proof rail that still needed backend-shaped project semantics onto `scripts/fixtures/backend/reference-backend/`.

T01 introduced `compiler/meshc/tests/support/m051_reference_backend.rs` as the canonical retained-fixture helper layer and rebased the Rust-side rails onto it. `compiler/meshc/tests/e2e_lsp.rs`, `compiler/meshc/tests/tooling_e2e.rs`, and `compiler/mesh-lsp/src/analysis.rs` now open the retained fixture instead of repo-root `reference-backend/`, and the truthful `meshc test` contract now expects the retained fixture’s `2 passed` summary. Formatter proof was cut over honestly: `compiler/meshc/tests/e2e_fmt.rs` and `compiler/mesh-fmt/src/lib.rs` now prove only the retained canonical files and subtrees that are actually formatter-clean today, while the slice-owned `compiler/meshc/tests/e2e_m051_s03.rs` fail-closes on stale repo-root paths and on any attempt to silently broaden the formatter target.

T02 moved the editor-host rails and shared syntax corpus to the same retained proof root. `tools/editors/vscode-mesh/src/test/suite/extension.test.ts` now opens retained `api/health.mpl` and `api/jobs.mpl`, preserves clean diagnostics/hover/same-file definition expectations, and still materializes the override-entry fixture under `.tmp/m036-s03/vscode-smoke/workspace/` so manifest-first root detection stays truthful. `tools/editors/neovim-mesh/tests/smoke.lua` now uses the retained backend root for the backend-shaped LSP case while preserving the override-entry and standalone boundaries. `scripts/fixtures/m036-s01-syntax-corpus.json` keeps its fixed `m036-s01-syntax-corpus-v1` / 15-case contract, but the backend-shaped interpolation case now points at the retained fixture instead of the repo-root compatibility copy.

T03 completed the public/internal contract split and added the slice-owned assembled replay. The VS Code and Neovim READMEs now talk about a generic backend-shaped proof surface rather than teaching repo-root `reference-backend/` or the internal retained fixture path. `scripts/tests/verify-m036-s03-contract.test.mjs` now fail-closes on leaked retained-fixture paths or stale repo-root backend wording. `scripts/verify-m051-s03.sh` became the authoritative S03 replay: it serially runs the docs contract, Rust retained-fixture rails, VS Code smoke, Neovim syntax/LSP replays, and the historical `scripts/verify-m036-s03.sh` wrapper, then publishes phase markers plus a copied retained bundle under `.tmp/m051-s03/verify/` instead of mutating the M036 trees in place.

The net effect is that the bounded tooling/editor proof no longer depends on repo-root `reference-backend/`, but it also does not promote the retained fixture into a new public workflow. Downstream slices now inherit a stable internal proof root, a public README contract that stays generic, and one assembled verifier/bundle for the whole cutover.

## Verification

Passed the slice-owned contract rail, all listed leaf commands from the plan, and the assembled replay:

- `cargo test -p meshc --test e2e_m051_s03 -- --nocapture`
- `cargo test -p meshc --test e2e_lsp lsp_json_rpc_reference_backend_flow -- --nocapture`
- `cargo test -p meshc --test tooling_e2e test_test_reference_backend_project_directory_succeeds -- --nocapture`
- `cargo test -p meshc --test tooling_e2e test_test_coverage_reports_unsupported_contract -- --nocapture`
- `cargo test -p meshc --test e2e_fmt fmt_check_reference_backend_directory_succeeds -- --nocapture`
- `cargo test -p mesh-lsp analyze_reference_backend_jobs_uses_project_imports -- --nocapture`
- `cargo test -p mesh-fmt reference_backend -- --nocapture`
- `node --test scripts/tests/verify-m036-s03-contract.test.mjs`
- `npm --prefix tools/editors/vscode-mesh run test:smoke`
- `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`
- `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp`
- `bash scripts/verify-m051-s03.sh`

The slice-specific observability surface also worked as designed after the assembled replay:

- `.tmp/m051-s03/verify/status.txt` = `ok`
- `.tmp/m051-s03/verify/current-phase.txt` = `complete`
- `.tmp/m051-s03/verify/phase-report.txt` contained passed markers for every named phase
- `.tmp/m051-s03/verify/latest-proof-bundle.txt` pointed at the retained copied bundle

## Requirements Advanced

- R119 — Removes the tooling/editor/LSP/formatter dependency on repo-root `reference-backend/` by moving those bounded rails to the retained backend fixture, which is a prerequisite for deleting the legacy app while keeping Mesher as the maintained deeper reference surface.

## Requirements Validated

None.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

None.

## Known Limitations

- The full retained backend fixture is still not formatter-clean. `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` remains an intentional known-red boundary, so the green formatter acceptance target stays bounded to the retained `api/` subtree plus canonical formatter-crate fixtures.
- The public editor-host proof remains intentionally bounded to clean diagnostics, hover, same-file definition inside backend-shaped project code, manifest-first override-entry rooting, and honest single-file fallback. This slice did not expand the public claim to broader editor-host behavior.
- Public docs/scaffold/skills outside the editor READMEs still need their own retarget away from repo-root `reference-backend/`; that is S04/S05 work, not part of this slice.

## Follow-ups

- S04 should audit the remaining public docs, scaffold output, and skill surfaces for stale repo-root `reference-backend/` teaching or leaked retained-fixture paths, keeping public wording generic while proof rails stay internal.
- S05 should delete the repo-root `reference-backend/` compatibility copy only after the remaining public/retained consumers are cut over.
- If a future slice wants the formatter rail to cover more of the retained backend fixture, fix `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` formatter debt first instead of widening the acceptance target and calling the red file green.

## Files Created/Modified

- `compiler/meshc/tests/support/m051_reference_backend.rs` — Added the canonical retained-backend fixture helper layer for Rust tests and retained proof utilities.
- `compiler/meshc/tests/e2e_m051_s03.rs` — Added the slice-owned contract target that fail-closes on stale repo-root paths, bounded formatter drift, editor/corpus retarget drift, README wording drift, and assembled verifier drift.
- `compiler/meshc/tests/e2e_lsp.rs` — Repointed the JSON-RPC LSP transport rail to retained backend files while preserving hover, same-file definition, formatting, and override-entry coverage.
- `compiler/meshc/tests/tooling_e2e.rs` — Repointed `meshc test` and coverage-contract tooling rails to the retained backend fixture and updated the truthful `2 passed` expectation.
- `compiler/meshc/tests/e2e_fmt.rs` — Bounded formatter acceptance to retained canonical files/subtrees instead of the repo-root compatibility copy.
- `compiler/mesh-lsp/src/analysis.rs` — Retargeted backend-shaped project analysis fixtures to the retained backend path and kept project-aware diagnostics anchored on the owning file.
- `compiler/mesh-fmt/src/lib.rs` — Moved formatter crate canonical retained-file fixtures to the retained backend path.
- `tools/editors/vscode-mesh/src/test/suite/extension.test.ts` — Retargeted the VS Code smoke suite to retained backend files while preserving clean diagnostics, hover, same-file definition, and override-entry proof.
- `tools/editors/neovim-mesh/tests/smoke.lua` — Retargeted the Neovim smoke to the retained backend fixture while preserving manifest-first override-entry and honest single-file behavior.
- `scripts/fixtures/m036-s01-syntax-corpus.json` — Moved the shared backend-shaped interpolation corpus case onto the retained backend fixture without changing the fixed corpus version/case count contract.
- `tools/editors/vscode-mesh/README.md` — Rewrote the public VS Code README around a generic backend-shaped proof surface and retained verifier handoff.
- `tools/editors/neovim-mesh/README.md` — Rewrote the public Neovim README around a generic backend-shaped proof surface and retained verifier handoff.
- `scripts/tests/verify-m036-s03-contract.test.mjs` — Strengthened the editor/docs contract test to fail closed on stale repo-root backend wording or leaked retained-fixture paths.
- `scripts/verify-m051-s03.sh` — Added the authoritative assembled S03 replay with phase markers and retained copied proof-bundle output.
- `.gsd/PROJECT.md` — Updated the living project state to reflect that M051/S03 is now complete and S04/S05 remain.
- `.gsd/KNOWLEDGE.md` — Recorded the new retained-tooling/editor gotchas that future slices are likely to trip.
- `.gsd/DECISIONS.md` — Recorded the public/internal editor-proof wording decision for downstream docs work.
