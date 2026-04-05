---
id: T03
parent: S03
milestone: M051
provides: []
requires: []
affects: []
key_files: ["tools/editors/vscode-mesh/README.md", "tools/editors/neovim-mesh/README.md", "scripts/tests/verify-m036-s03-contract.test.mjs", "compiler/meshc/tests/e2e_m051_s03.rs", "scripts/verify-m051-s03.sh", "scripts/verify-m034-s04-extension.sh", ".gsd/KNOWLEDGE.md", ".gsd/milestones/M051/slices/S03/tasks/T03-SUMMARY.md"]
key_decisions: ["Keep the public editor READMEs generic about a backend-shaped proof surface and move retained-fixture specifics into repo-owned verifier rails instead of teaching the internal fixture as a public workflow.", "Let `scripts/verify-m051-s03.sh` resolve `nvim` from PATH and materialize the historical vendored path expected by `scripts/verify-m036-s03.sh`, so the old wrapper can replay truthfully without manual environment setup.", "Make the nested `scripts/verify-m034-s04-extension.sh` LSP gate fail closed on a non-zero `e2e_lsp` test count instead of the stale exact `running 1 test` assumption."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Ran `node --test scripts/tests/verify-m036-s03-contract.test.mjs`, `cargo test -p meshc --test e2e_m051_s03 -- --nocapture`, and `bash scripts/verify-m051-s03.sh`; all passed on the final tree. Verified the new observability surfaces directly: `.tmp/m051-s03/verify/status.txt` is `ok`, `.tmp/m051-s03/verify/current-phase.txt` is `complete`, `.tmp/m051-s03/verify/phase-report.txt` shows every named phase as passed, and `.tmp/m051-s03/verify/latest-proof-bundle.txt` points at the copied retained proof bundle."
completed_at: 2026-04-04T16:38:15.035Z
blocker_discovered: false
---

# T03: Added bounded editor README guards and the assembled M051/S03 tooling replay with retained proof bundles.

> Added bounded editor README guards and the assembled M051/S03 tooling replay with retained proof bundles.

## What Happened
---
id: T03
parent: S03
milestone: M051
key_files:
  - tools/editors/vscode-mesh/README.md
  - tools/editors/neovim-mesh/README.md
  - scripts/tests/verify-m036-s03-contract.test.mjs
  - compiler/meshc/tests/e2e_m051_s03.rs
  - scripts/verify-m051-s03.sh
  - scripts/verify-m034-s04-extension.sh
  - .gsd/KNOWLEDGE.md
  - .gsd/milestones/M051/slices/S03/tasks/T03-SUMMARY.md
key_decisions:
  - Keep the public editor READMEs generic about a backend-shaped proof surface and move retained-fixture specifics into repo-owned verifier rails instead of teaching the internal fixture as a public workflow.
  - Let `scripts/verify-m051-s03.sh` resolve `nvim` from PATH and materialize the historical vendored path expected by `scripts/verify-m036-s03.sh`, so the old wrapper can replay truthfully without manual environment setup.
  - Make the nested `scripts/verify-m034-s04-extension.sh` LSP gate fail closed on a non-zero `e2e_lsp` test count instead of the stale exact `running 1 test` assumption.
duration: ""
verification_result: passed
completed_at: 2026-04-04T16:38:15.042Z
blocker_discovered: false
---

# T03: Added bounded editor README guards and the assembled M051/S03 tooling replay with retained proof bundles.

**Added bounded editor README guards and the assembled M051/S03 tooling replay with retained proof bundles.**

## What Happened

Rewrote the public VS Code and Neovim READMEs so they describe a small backend-shaped proof surface generically instead of naming repo-root `reference-backend/` paths or leaking the retained fixture path as a public workflow. Strengthened `scripts/tests/verify-m036-s03-contract.test.mjs` to fail closed on stale repo-root editor proof wording and retained-fixture path leakage, and extended `compiler/meshc/tests/e2e_m051_s03.rs` with source-level guards for the README wording plus the new slice verifier contract. Implemented `scripts/verify-m051-s03.sh` as the authoritative assembled replay for this slice: it replays the M036 editor/docs contract test, the M051 Rust source contract target, direct VS Code smoke, direct Neovim syntax and LSP rails, and the historical `scripts/verify-m036-s03.sh` wrapper, then publishes `status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, and `latest-proof-bundle.txt` under `.tmp/m051-s03/verify/`. The retained bundle copies the delegated `.tmp/m036-s02/` and `.tmp/m036-s03/` trees plus the fresh timestamped `.tmp/m051-s03/` Rust-contract artifacts. A targeted wrapper-side fix in `scripts/verify-m034-s04-extension.sh` was also required so the nested extension proof now checks `e2e_lsp` for a non-zero test count instead of the stale exact `running 1 test` assumption.

## Verification

Ran `node --test scripts/tests/verify-m036-s03-contract.test.mjs`, `cargo test -p meshc --test e2e_m051_s03 -- --nocapture`, and `bash scripts/verify-m051-s03.sh`; all passed on the final tree. Verified the new observability surfaces directly: `.tmp/m051-s03/verify/status.txt` is `ok`, `.tmp/m051-s03/verify/current-phase.txt` is `complete`, `.tmp/m051-s03/verify/phase-report.txt` shows every named phase as passed, and `.tmp/m051-s03/verify/latest-proof-bundle.txt` points at the copied retained proof bundle.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `node --test scripts/tests/verify-m036-s03-contract.test.mjs` | 0 | ✅ pass | 785ms |
| 2 | `cargo test -p meshc --test e2e_m051_s03 -- --nocapture` | 0 | ✅ pass | 10200ms |
| 3 | `bash scripts/verify-m051-s03.sh` | 0 | ✅ pass | 124000ms |


## Deviations

Updated `scripts/verify-m034-s04-extension.sh` even though it was outside the planned output list because the historical M036 wrapper still assumed `cargo test -q -p meshc --test e2e_lsp -- --nocapture` would print `running 1 test`. Without that targeted verifier fix, the assembled replay failed red for stale wrapper plumbing rather than for real extension/LSP drift.

## Known Issues

None.

## Files Created/Modified

- `tools/editors/vscode-mesh/README.md`
- `tools/editors/neovim-mesh/README.md`
- `scripts/tests/verify-m036-s03-contract.test.mjs`
- `compiler/meshc/tests/e2e_m051_s03.rs`
- `scripts/verify-m051-s03.sh`
- `scripts/verify-m034-s04-extension.sh`
- `.gsd/KNOWLEDGE.md`
- `.gsd/milestones/M051/slices/S03/tasks/T03-SUMMARY.md`


## Deviations
Updated `scripts/verify-m034-s04-extension.sh` even though it was outside the planned output list because the historical M036 wrapper still assumed `cargo test -q -p meshc --test e2e_lsp -- --nocapture` would print `running 1 test`. Without that targeted verifier fix, the assembled replay failed red for stale wrapper plumbing rather than for real extension/LSP drift.

## Known Issues
None.
