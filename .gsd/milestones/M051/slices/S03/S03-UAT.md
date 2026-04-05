# S03: Migrate tooling and editor rails to a bounded backend fixture — UAT

**Milestone:** M051
**Written:** 2026-04-04T16:49:39.421Z

# S03: Migrate tooling and editor rails to a bounded backend fixture — UAT

**Milestone:** M051
**Written:** 2026-04-04

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice changed proof rails, contract wording, and retained verifier plumbing. The authoritative evidence is the repo-owned command suite and the retained logs/bundle markers it writes, not a human-only product walkthrough.

## Preconditions

- Run from the repository root.
- `cargo`, `node`, `npm`, `python3`, `bash`, and `nvim` are installed and available.
- The local host can build `target/debug/meshc` and run the VS Code Extension Development Host smoke used by `tools/editors/vscode-mesh`.
- No manual setup inside `scripts/fixtures/backend/reference-backend/` is required; the retained fixture must stay source-only.

## Smoke Test

1. Run `bash scripts/verify-m051-s03.sh`.
2. Open `.tmp/m051-s03/verify/status.txt`.
3. Open `.tmp/m051-s03/verify/current-phase.txt` and `.tmp/m051-s03/verify/latest-proof-bundle.txt`.
4. **Expected:** the script exits `0`, `status.txt` contains `ok`, `current-phase.txt` contains `complete`, and `latest-proof-bundle.txt` points at a retained proof bundle directory.

## Test Cases

### 1. Slice-owned retained-fixture contract rail

1. Run `cargo test -p meshc --test e2e_m051_s03 -- --nocapture`.
2. Inspect the passing output and, if desired, the fresh `.tmp/m051-s03/` artifact directories it creates.
3. **Expected:** the target runs real tests, the helper paths resolve under `scripts/fixtures/backend/reference-backend/`, the source rails omit repo-root `reference-backend/`, the bounded formatter contract stays honest, the editor/corpus path retarget stays fixed, and the assembled verifier source still advertises every required phase and retained bundle marker.

### 2. Leaf Rust tooling, formatter, and analysis rails

1. Run `cargo test -p meshc --test e2e_lsp lsp_json_rpc_reference_backend_flow -- --nocapture`.
2. Run `cargo test -p meshc --test tooling_e2e test_test_reference_backend_project_directory_succeeds -- --nocapture`.
3. Run `cargo test -p meshc --test tooling_e2e test_test_coverage_reports_unsupported_contract -- --nocapture`.
4. Run `cargo test -p meshc --test e2e_fmt fmt_check_reference_backend_directory_succeeds -- --nocapture`.
5. Run `cargo test -p mesh-lsp analyze_reference_backend_jobs_uses_project_imports -- --nocapture`.
6. Run `cargo test -p mesh-fmt reference_backend -- --nocapture`.
7. **Expected:** every command exits `0`; the retained backend fixture is the proof root; `meshc test scripts/fixtures/backend/reference-backend` reports `2 passed`; the coverage rail remains an explicit unsupported contract rather than a green placeholder; and the bounded formatter contract stays silent and green on the retained canonical files/subtree.

### 3. VS Code smoke uses the retained fixture and keeps the override-entry proof

1. Run `npm --prefix tools/editors/vscode-mesh run test:smoke`.
2. Open `.tmp/m036-s03/vscode-smoke/smoke.log`.
3. Confirm the log mentions the retained backend fixture root, retained `api/health.mpl` / `api/jobs.mpl`, and the materialized override-entry fixture under `.tmp/m036-s03/vscode-smoke/workspace/override-entry-project/`.
4. **Expected:** the Extension Development Host smoke passes, logs `Using retained backend fixture root .../scripts/fixtures/backend/reference-backend`, reports clean diagnostics on retained health/jobs plus override-entry files, resolves hover/definition inside retained `api/jobs.mpl`, and keeps the override-entry hover proof green.

### 4. Neovim syntax and LSP smokes use the retained fixture without broadening scope

1. Run `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`.
2. Open `.tmp/m036-s02/syntax/neovim-smoke.log` and `.tmp/m036-s02/syntax/corpus/materialized-corpus.json`.
3. Run `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp`.
4. Open `.tmp/m036-s02/lsp/neovim-smoke.log` and `.tmp/m036-s02/lsp/upstream-lsp.log`.
5. **Expected:** the syntax replay keeps the fixed 15-case corpus contract and shows the `reference-backend-http-port-log` case against `scripts/fixtures/backend/reference-backend/main.mpl`; the LSP replay attaches one client to retained backend `api/health.mpl` / `api/jobs.mpl`, keeps manifest-first override-entry rooting, and still proves honest standalone single-file mode.

### 5. Public editor docs stay generic while the assembled verifier retains the migrated evidence

1. Run `node --test scripts/tests/verify-m036-s03-contract.test.mjs`.
2. Run `bash scripts/verify-m051-s03.sh`.
3. Open `.tmp/m051-s03/verify/phase-report.txt`, `.tmp/m051-s03/verify/full-contract.log`, and the directory named by `.tmp/m051-s03/verify/latest-proof-bundle.txt`.
4. **Expected:** the Node contract test passes its mutation/fail-closed cases; the assembled verifier exits `0`; `phase-report.txt` shows every named phase as `passed`; and the retained bundle contains copied `retained-m036-s02-*`, `retained-m036-s03-*`, and fresh `retained-m051-s03-artifacts/` evidence instead of pointing back into mutable historical trees.

## Edge Cases

### Explicit unsupported coverage contract stays honest

1. Run `cargo test -p meshc --test tooling_e2e test_test_coverage_reports_unsupported_contract -- --nocapture`.
2. **Expected:** the cargo test target passes because the inner `meshc test --coverage scripts/fixtures/backend/reference-backend` command still exits with the explicit unsupported coverage message rather than silently succeeding.

### Bounded formatter contract does not overclaim the whole retained fixture

1. Run `cargo test -p meshc --test e2e_m051_s03 m051_s03_formatter_contract_stays_bounded_to_retained_api_subtree -- --nocapture`.
2. **Expected:** the test proves the retained `api/` subtree is green, but also confirms `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` remains intentionally known-red for formatter purposes instead of being silently absorbed into the acceptance target.

## Failure Signals

- `node --test scripts/tests/verify-m036-s03-contract.test.mjs` fails on leaked `reference-backend/` or `scripts/fixtures/backend/reference-backend` wording in the public editor READMEs.
- `npm --prefix tools/editors/vscode-mesh run test:smoke` stops logging the retained backend fixture root, loses clean diagnostics, or stops proving same-file definition in retained `api/jobs.mpl`.
- `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp` reports the wrong root marker, duplicate clients, or a stale repo-root backend path.
- `bash scripts/verify-m051-s03.sh` leaves `status.txt != ok`, `current-phase.txt != complete`, or a phase missing from `phase-report.txt`.

## Requirements Proved By This UAT

- R119 (partial) — the tooling/editor/LSP/formatter portion of the `reference-backend/` retirement is now off the repo-root compatibility copy and onto the retained backend fixture, which is a necessary step toward Mesher becoming the maintained deeper reference app.

## Not Proven By This UAT

- Public docs/scaffold/skill retarget away from repo-root `reference-backend/` outside the editor-host surfaces; that is S04 work.
- Final deletion of the repo-root `reference-backend/` compatibility copy; that is S05 work.
- Full formatter cleanliness for every file in the retained backend fixture root beyond the bounded canonical acceptance surface.

## Notes for Tester

Use `.tmp/m051-s03/verify/phase-report.txt` as the first stop on any failure. If the assembled verifier fails after the editor-host phases, follow `.tmp/m051-s03/verify/latest-proof-bundle.txt` into the copied retained bundle rather than debugging the mutable `.tmp/m036-s02/` or `.tmp/m036-s03/` trees directly. The public README wording is intentionally generic; do not treat the internal retained fixture path as a new public workflow just because the proofs use it.
