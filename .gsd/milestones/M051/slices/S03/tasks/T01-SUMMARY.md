---
id: T01
parent: S03
milestone: M051
provides: []
requires: []
affects: []
key_files: ["compiler/meshc/tests/support/m051_reference_backend.rs", "compiler/meshc/tests/e2e_lsp.rs", "compiler/meshc/tests/tooling_e2e.rs", "compiler/meshc/tests/e2e_fmt.rs", "compiler/mesh-lsp/src/analysis.rs", "compiler/mesh-fmt/src/lib.rs", "compiler/meshc/tests/e2e_m051_s03.rs", ".gsd/milestones/M051/slices/S03/tasks/T01-SUMMARY.md"]
key_decisions: ["Bound the formatter acceptance contract to the retained fixture `api/` subtree instead of the full retained backend root because `tests/fixture.test.mpl` is still intentionally unformatted.", "Added a slice-owned `e2e_m051_s03` contract target that mixes source assertions with direct `meshc test` and `meshc fmt --check` probes so stale repo-root path usage fails closed immediately."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Task-local verification passed for all planned T01 rails: `cargo test -p meshc --test e2e_m051_s03 -- --nocapture`, `cargo test -p meshc --test e2e_lsp lsp_json_rpc_reference_backend_flow -- --nocapture`, `cargo test -p meshc --test tooling_e2e test_test_reference_backend_project_directory_succeeds -- --nocapture`, `cargo test -p meshc --test tooling_e2e test_test_coverage_reports_unsupported_contract -- --nocapture`, `cargo test -p meshc --test e2e_fmt fmt_check_reference_backend_directory_succeeds -- --nocapture`, `cargo test -p mesh-lsp analyze_reference_backend_jobs_uses_project_imports -- --nocapture`, and `cargo test -p mesh-fmt reference_backend -- --nocapture` all passed against the retained fixture. Slice-level visibility was also checked: `node --test scripts/tests/verify-m036-s03-contract.test.mjs` passed, while `bash scripts/verify-m051-s03.sh` failed with exit 127 because T03 has not created that verifier yet."
completed_at: 2026-04-04T15:46:17.618Z
blocker_discovered: false
---

# T01: Retargeted the Rust tooling, LSP, and bounded formatter rails to the retained backend fixture and added a slice-owned stale-path contract test.

> Retargeted the Rust tooling, LSP, and bounded formatter rails to the retained backend fixture and added a slice-owned stale-path contract test.

## What Happened
---
id: T01
parent: S03
milestone: M051
key_files:
  - compiler/meshc/tests/support/m051_reference_backend.rs
  - compiler/meshc/tests/e2e_lsp.rs
  - compiler/meshc/tests/tooling_e2e.rs
  - compiler/meshc/tests/e2e_fmt.rs
  - compiler/mesh-lsp/src/analysis.rs
  - compiler/mesh-fmt/src/lib.rs
  - compiler/meshc/tests/e2e_m051_s03.rs
  - .gsd/milestones/M051/slices/S03/tasks/T01-SUMMARY.md
key_decisions:
  - Bound the formatter acceptance contract to the retained fixture `api/` subtree instead of the full retained backend root because `tests/fixture.test.mpl` is still intentionally unformatted.
  - Added a slice-owned `e2e_m051_s03` contract target that mixes source assertions with direct `meshc test` and `meshc fmt --check` probes so stale repo-root path usage fails closed immediately.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T15:46:17.630Z
blocker_discovered: false
---

# T01: Retargeted the Rust tooling, LSP, and bounded formatter rails to the retained backend fixture and added a slice-owned stale-path contract test.

**Retargeted the Rust tooling, LSP, and bounded formatter rails to the retained backend fixture and added a slice-owned stale-path contract test.**

## What Happened

Extended `compiler/meshc/tests/support/m051_reference_backend.rs` with retained-fixture helpers for the bounded API subtree, canonical retained files, and retained test files. Repointed the Rust-side leaf rails in `compiler/meshc/tests/e2e_lsp.rs`, `compiler/meshc/tests/tooling_e2e.rs`, `compiler/meshc/tests/e2e_fmt.rs`, `compiler/mesh-lsp/src/analysis.rs`, and `compiler/mesh-fmt/src/lib.rs` away from the repo-root compatibility copy and onto `scripts/fixtures/backend/reference-backend`, including the truthful `meshc test` expectation change from `1 passed` to `2 passed`. Added `compiler/meshc/tests/e2e_m051_s03.rs` as a slice-owned fail-closed contract rail that verifies the retained helper paths, asserts stale repo-root path usage is gone, proves the two-file `meshc test` summary, and keeps the formatter boundary explicit by leaving `tests/fixture.test.mpl` red while the retained `api/` subtree stays green.

## Verification

Task-local verification passed for all planned T01 rails: `cargo test -p meshc --test e2e_m051_s03 -- --nocapture`, `cargo test -p meshc --test e2e_lsp lsp_json_rpc_reference_backend_flow -- --nocapture`, `cargo test -p meshc --test tooling_e2e test_test_reference_backend_project_directory_succeeds -- --nocapture`, `cargo test -p meshc --test tooling_e2e test_test_coverage_reports_unsupported_contract -- --nocapture`, `cargo test -p meshc --test e2e_fmt fmt_check_reference_backend_directory_succeeds -- --nocapture`, `cargo test -p mesh-lsp analyze_reference_backend_jobs_uses_project_imports -- --nocapture`, and `cargo test -p mesh-fmt reference_backend -- --nocapture` all passed against the retained fixture. Slice-level visibility was also checked: `node --test scripts/tests/verify-m036-s03-contract.test.mjs` passed, while `bash scripts/verify-m051-s03.sh` failed with exit 127 because T03 has not created that verifier yet.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m051_s03 -- --nocapture` | 0 | ✅ pass | 11764ms |
| 2 | `cargo test -p meshc --test e2e_lsp lsp_json_rpc_reference_backend_flow -- --nocapture` | 0 | ✅ pass | 8773ms |
| 3 | `cargo test -p meshc --test tooling_e2e test_test_reference_backend_project_directory_succeeds -- --nocapture` | 0 | ✅ pass | 9803ms |
| 4 | `cargo test -p meshc --test tooling_e2e test_test_coverage_reports_unsupported_contract -- --nocapture` | 0 | ✅ pass | 5363ms |
| 5 | `cargo test -p meshc --test e2e_fmt fmt_check_reference_backend_directory_succeeds -- --nocapture` | 0 | ✅ pass | 6588ms |
| 6 | `cargo test -p mesh-lsp analyze_reference_backend_jobs_uses_project_imports -- --nocapture` | 0 | ✅ pass | 852ms |
| 7 | `cargo test -p mesh-fmt reference_backend -- --nocapture` | 0 | ✅ pass | 240ms |
| 8 | `node --test scripts/tests/verify-m036-s03-contract.test.mjs` | 0 | ✅ pass | 1261ms |
| 9 | `bash scripts/verify-m051-s03.sh` | 127 | ❌ fail | 15ms |


## Deviations

None.

## Known Issues

`bash scripts/verify-m051-s03.sh` still fails with exit 127 because T03 owns that assembled slice verifier and the file does not exist yet. `node --test scripts/tests/verify-m036-s03-contract.test.mjs` already passes, so the remaining slice-level red rail is expected task sequencing rather than a blocker.

## Files Created/Modified

- `compiler/meshc/tests/support/m051_reference_backend.rs`
- `compiler/meshc/tests/e2e_lsp.rs`
- `compiler/meshc/tests/tooling_e2e.rs`
- `compiler/meshc/tests/e2e_fmt.rs`
- `compiler/mesh-lsp/src/analysis.rs`
- `compiler/mesh-fmt/src/lib.rs`
- `compiler/meshc/tests/e2e_m051_s03.rs`
- `.gsd/milestones/M051/slices/S03/tasks/T01-SUMMARY.md`


## Deviations
None.

## Known Issues
`bash scripts/verify-m051-s03.sh` still fails with exit 127 because T03 owns that assembled slice verifier and the file does not exist yet. `node --test scripts/tests/verify-m036-s03-contract.test.mjs` already passes, so the remaining slice-level red rail is expected task sequencing rather than a blocker.
