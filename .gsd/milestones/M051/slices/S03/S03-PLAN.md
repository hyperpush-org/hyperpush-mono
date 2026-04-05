# S03: Migrate tooling and editor rails to a bounded backend fixture

**Goal:** Move the remaining tooling, formatter, LSP, and editor proof rails off the repo-root `reference-backend/` compatibility copy and onto the retained backend fixture, while keeping the public tooling story bounded and examples-first instead of turning the internal fixture into a new onboarding path.
**Demo:** After this: LSP, tooling, formatter, and editor smoke rails replay against a small retained backend-shaped fixture instead of `reference-backend/`, preserving the bounded project semantics those rails actually need.

## Tasks
- [x] **T01: Retargeted the Rust tooling, LSP, and bounded formatter rails to the retained backend fixture and added a slice-owned stale-path contract test.** — Rebind the Rust-side tooling and formatter proof rails to the retained backend fixture and introduce the slice-owned contract target early so the migration has a dedicated fail-closed seam from the first task.

### Why
The Rust-side `meshc`, `mesh-lsp`, and `mesh-fmt` rails still hardcode repo-root `reference-backend/`. This task moves those bounded proofs onto the retained S02 fixture, fixes the truthful `meshc test` summary change, and makes the formatter boundary explicit so later tasks do not silently absorb known `tests/fixture.test.mpl` formatter debt.

### Steps
1. Extend `compiler/meshc/tests/support/m051_reference_backend.rs` with any retained-fixture path helpers the meshc-side tests need.
2. Repoint `compiler/meshc/tests/e2e_lsp.rs`, `compiler/meshc/tests/tooling_e2e.rs`, and `compiler/mesh-lsp/src/analysis.rs` from repo-root `reference-backend/` to the retained fixture and update the truthful `meshc test` expectation from `1 passed` to `2 passed`.
3. Replace the repo-root formatter directory/file targets in `compiler/meshc/tests/e2e_fmt.rs` and `compiler/mesh-fmt/src/lib.rs` with a bounded retained-fixture contract that stays green today rather than claiming the full retained root is canonical.
4. Create `compiler/meshc/tests/e2e_m051_s03.rs` with fail-closed assertions for the new retained-fixture paths and the bounded formatter choice.

### Must-Haves
- [ ] No Rust-side tooling/LSP/formatter rail still opens repo-root `reference-backend/` as the bounded proof root.
- [ ] `tooling_e2e` proves `meshc test` against the retained fixture and expects the truthful `2 passed` summary.
- [ ] The formatter contract stays bounded to canonical retained files/subtrees and does not pretend `tests/fixture.test.mpl` is green.
- [ ] `compiler/meshc/tests/e2e_m051_s03.rs` runs more than zero real tests after the retarget.

### Failure Modes
| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `meshc` CLI integration rails | Fail closed with the retained path and expected command in the assertion text; do not silently fall back to repo-root `reference-backend/` | Preserve the existing test harness timeout and report the stale phase/path | Treat unexpected summary/output text as contract drift and archive it in the slice-owned contract target |
| JSON-RPC LSP harness | Keep opened URIs and retained paths in artifacts so wrong-root regressions are attributable | Fail the test instead of retrying hiddenly | Treat wrong hover/definition/diagnostics payloads as explicit drift rather than acceptable nulls |
| Formatter CLI/library rails | Fail on non-silent `fmt --check` or rewritten canonical files | N/A for the bounded file/unit rails | Treat a red full-root retained fixture as scope drift, not as a reason to broaden the acceptance target |

### Load Profile
- **Shared resources**: Cargo target directory, temporary test workspaces, retained fixture source tree, and LSP subprocesses.
- **Per-operation cost**: one `meshc` subprocess or one cargo test target per rail plus bounded file reads.
- **10x breakpoint**: repeated full-target cargo replays waste time first; the code path itself is not load-sensitive, but stale artifacts and slow reruns are.

### Negative Tests
- **Malformed inputs**: wrong project root, orphaned file target, stale repo-root `reference-backend/` path, and intentionally unformatted retained backend text that should still type-check before formatting.
- **Error paths**: bogus import diagnostics, wrong definition target, wrong `meshc test` pass count, or `fmt --check` touching the wrong retained files.
- **Boundary conditions**: same-file definition remains same-file, formatter proof remains limited to canonical retained files, and the repo-root compatibility copy is no longer the proof root.

### Done when
`compiler/meshc/tests/e2e_m051_s03.rs` catches stale repo-root path usage and the retargeted Rust rails pass against the retained backend fixture without broadening the formatter contract.
  - Estimate: 2h
  - Files: compiler/meshc/tests/support/m051_reference_backend.rs, compiler/meshc/tests/e2e_lsp.rs, compiler/meshc/tests/tooling_e2e.rs, compiler/meshc/tests/e2e_fmt.rs, compiler/mesh-lsp/src/analysis.rs, compiler/mesh-fmt/src/lib.rs, compiler/meshc/tests/e2e_m051_s03.rs
  - Verify: `cargo test -p meshc --test e2e_m051_s03 -- --nocapture`
`cargo test -p meshc --test e2e_lsp lsp_json_rpc_reference_backend_flow -- --nocapture`
`cargo test -p meshc --test tooling_e2e test_test_reference_backend_project_directory_succeeds -- --nocapture`
`cargo test -p meshc --test tooling_e2e test_test_coverage_reports_unsupported_contract -- --nocapture`
`cargo test -p meshc --test e2e_fmt fmt_check_reference_backend_directory_succeeds -- --nocapture`
`cargo test -p mesh-lsp analyze_reference_backend_jobs_uses_project_imports -- --nocapture`
`cargo test -p mesh-fmt reference_backend -- --nocapture`
- [x] **T02: Retargeted the VS Code and Neovim smoke rails plus the shared syntax corpus to the retained backend fixture.** — Move the editor-host smoke rails and the shared syntax corpus onto the retained backend fixture without changing the bounded behaviors they already prove.

### Why
The editor-host rails only need a small manifest-rooted backend project with stable line numbers. S02 already preserved that in the retained fixture, so this task is a path cutover, not a harness redesign. Doing it after T01 keeps Neovim’s upstream-LSP replay aligned with the retargeted Rust rails.

### Steps
1. Repoint the VS Code smoke fixture paths in `tools/editors/vscode-mesh/src/test/suite/extension.test.ts` to the retained backend fixture while preserving same-file hover/definition expectations.
2. Repoint the Neovim smoke fixture paths and expected project root in `tools/editors/neovim-mesh/tests/smoke.lua` to the retained backend fixture while preserving the override-entry and single-file cases.
3. Move the backend-shaped interpolation corpus case in `scripts/fixtures/m036-s01-syntax-corpus.json` to the retained fixture path without changing the contract version or case count.
4. Extend the slice-owned contract target if needed so the editor/corpus path retarget fails closed instead of silently drifting back to repo-root `reference-backend/`.

### Must-Haves
- [ ] VS Code smoke opens retained `api/health.mpl` and `api/jobs.mpl` and still proves clean diagnostics, hover, and same-file definition.
- [ ] Neovim smoke reuses one client against the retained backend root and keeps the override-entry and single-file checks unchanged.
- [ ] The shared syntax corpus stays at contract version `m036-s01-syntax-corpus-v1` with 15 cases.
- [ ] No editor-host or corpus rail broadens its scope to Mesher or another larger project.

### Failure Modes
| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| VS Code Extension Development Host smoke | Fail the smoke run with the retained file path and phase label in the log | Treat startup/activation waits as hard failures after the existing bounded retry window | Treat empty hover/definition or wrong diagnostics as drift, not as acceptable editor-host flake |
| Neovim headless smoke | Preserve the failing phase (`syntax` or `lsp`) and root marker in the log | Keep the existing explicit attach timeout and fail closed when it expires | Treat wrong root/client-count results as contract drift rather than retrying into a green-looking run |
| Shared syntax corpus materializer | Fail if a case path or fenced snippet no longer resolves | N/A for the JSON/path swap itself | Treat changed line ranges or case-count drift as contract breakage |

### Load Profile
- **Shared resources**: editor-host temp workspaces, `.tmp/m036-s02/` artifact tree, `.tmp/m036-s03/` smoke tree, and compiled VS Code extension output.
- **Per-operation cost**: one extension compile/smoke run, one or two headless Neovim replays, and one corpus materialization pass.
- **10x breakpoint**: editor-host startup time dominates first; the retained fixture itself is tiny and not load-sensitive.

### Negative Tests
- **Malformed inputs**: missing retained fixture root, wrong root marker, stale corpus path, or missing override-entry fixture support file.
- **Error paths**: attach timeout, duplicate client, wrong same-file definition target, or corpus-materializer failure on a stale line range.
- **Boundary conditions**: same-file definition stays same-file, the override-entry root still wins over the repo `.git`, and the corpus case count/version stay unchanged.

### Done when
The repo-owned VS Code smoke, Neovim smoke, and shared syntax corpus all consume the retained backend fixture and still prove the same bounded editor behaviors they proved against repo-root `reference-backend/`.
  - Estimate: 1h30m
  - Files: tools/editors/vscode-mesh/src/test/suite/extension.test.ts, tools/editors/neovim-mesh/tests/smoke.lua, scripts/fixtures/m036-s01-syntax-corpus.json, compiler/meshc/tests/e2e_m051_s03.rs
  - Verify: `npm --prefix tools/editors/vscode-mesh run test:smoke`
`NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`
`NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp`
- [x] **T03: Added bounded editor README guards and the assembled M051/S03 tooling replay with retained proof bundles.** — Finish the slice with the public/internal wording guardrails and one authoritative S03 replay that proves the migrated rails together and publishes a retained bundle for downstream slices.

### Why
After T01 and T02, the leaf rails move, but the slice is not durable until the editor READMEs describe the migrated proof surface honestly and one slice-owned verifier replays the Rust rails, editor hosts, and historical wrappers together. This is also where R115–R118 and R127 need explicit protection so the internal retained fixture does not become a new public onboarding path.

### Steps
1. Rewrite the VS Code and Neovim READMEs so they describe a generic backend-shaped proof surface instead of naming repo-root `reference-backend/` or teaching the retained fixture path as a public workflow.
2. Strengthen `scripts/tests/verify-m036-s03-contract.test.mjs` so stale `reference-backend/` proof wording or accidental retained-fixture path leakage fails closed.
3. Extend `compiler/meshc/tests/e2e_m051_s03.rs` with final source-level guards for README/verifier drift and implement `scripts/verify-m051-s03.sh` as the phase-labeled assembled replay for the migrated rails.
4. Make `scripts/verify-m051-s03.sh` retain status, phase, full log, and bundle pointers under `.tmp/m051-s03/verify/`, and copy delegated `.tmp/m036-s02/` / `.tmp/m036-s03/` evidence instead of mutating those trees in place.

### Must-Haves
- [ ] Public editor READMEs stay generic about the backend-shaped proof surface and do not tell readers to use `scripts/fixtures/backend/reference-backend/` directly.
- [ ] `scripts/tests/verify-m036-s03-contract.test.mjs` fails closed on stale `reference-backend/` wording or internal retained-fixture path leakage in the editor READMEs.
- [ ] `scripts/verify-m051-s03.sh` is the authoritative slice replay and publishes `status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, and `latest-proof-bundle.txt` under `.tmp/m051-s03/verify/`.
- [ ] The assembled replay proves the migrated Rust rails, editor-host rails, and historical M036 wrapper story together from one named command.

### Failure Modes
| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| README/docs contract tests | Fail closed on the exact stale text or missing marker instead of silently tolerating drift | N/A for local source assertions | Treat leaked fixture paths or stale proof wording as contract failures |
| `scripts/verify-m051-s03.sh` phase runner | Stop on the first failing phase, write the phase marker, and preserve the phase log | Record timeout in the phase log and stop; do not continue to later phases | Treat missing status/bundle markers as verifier drift, not as a soft warning |
| Delegated historical wrappers | Copy their artifacts after successful runs and fail if the expected retained trees or pointers are missing | Respect wrapper timeouts rather than spinning in polling loops | Treat incomplete retained bundles as assembled-proof failure |

### Load Profile
- **Shared resources**: `.tmp/m051-s03/verify/`, delegated `.tmp/m036-s02/` and `.tmp/m036-s03/` trees, Node test runner, and shell subprocesses.
- **Per-operation cost**: one node contract test plus one assembled shell replay that serially replays the migrated rails.
- **10x breakpoint**: the assembled replay becomes the expensive step first; stale retained bundles or wrapper drift will dominate rerun time.

### Negative Tests
- **Malformed inputs**: stale README wording, missing phase markers, missing bundle pointer file, or leaked retained-fixture path in public editor prose.
- **Error paths**: a delegated wrapper exits green but leaves zero tests or missing retained artifacts, or the assembled verifier misses a first-failing phase.
- **Boundary conditions**: editor READMEs stay public and bounded, while the retained fixture remains internal and maintainer-facing.

### Done when
The editor READMEs describe the migrated proof surface honestly, the contract tests fail closed on stale wording, and `bash scripts/verify-m051-s03.sh` replays the migrated tooling/editor rails into one retained proof bundle.
  - Estimate: 2h
  - Files: tools/editors/vscode-mesh/README.md, tools/editors/neovim-mesh/README.md, scripts/tests/verify-m036-s03-contract.test.mjs, compiler/meshc/tests/e2e_m051_s03.rs, scripts/verify-m051-s03.sh
  - Verify: `node --test scripts/tests/verify-m036-s03-contract.test.mjs`
`cargo test -p meshc --test e2e_m051_s03 -- --nocapture`
`bash scripts/verify-m051-s03.sh`
