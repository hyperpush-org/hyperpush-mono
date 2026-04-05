# S05: Delete reference-backend and close the assembled acceptance rail

**Goal:** Remove the repo-root `reference-backend/` compatibility tree and retarget the last docs/verifier surfaces so the post-deletion repo proves Mesher, the retained backend fixture, migrated tooling rails, and the examples-first public story from stable top-level commands.
**Demo:** After this: The repo ships without `reference-backend/`, and the final acceptance bundle proves Mesher live runtime, retained backend proof, migrated tooling rails, and examples-first docs together on the post-deletion tree.

## Tasks
- [x] **T01: Moved the public proof-page verifier to scripts/ and retargeted the surviving docs and contract rails to the new canonical path.** — ---
estimated_steps: 4
estimated_files: 6
skills_used:
  - bash-scripting
  - vitepress
  - rust-testing
---

Relocate the public proof-page verifier from the retiring repo-root app tree to a stable top-level script, and retarget the direct positive callers before anything deletes `reference-backend/`.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `scripts/verify-production-proof-surface.sh` | stop on the first missing marker and name the drifting file or command | fail closed; do not silently skip proof-page checks | treat wrong commands, missing route markers, or stale repo-root paths as contract drift |
| Historical wrapper callers | keep each caller on the new path and fail closed if any `require_file` or command string still points at the old tree | use existing wrapper timeouts and stop on the first failing phase | treat mismatched command strings or copied-artifact paths as real verifier drift |
| Production Backend Proof page | preserve the same public route and contract while changing only the verifier path | N/A for source edits | treat stale verifier commands as public-doc drift |

## Load Profile

- **Shared resources**: the proof-page verifier script, wrapper artifacts under `.tmp/m050-s01/verify/` and `.tmp/m050-s03/verify/`, and the public proof-page source.
- **Per-operation cost**: one shell verifier move plus bounded Rust contract and wrapper-source updates.
- **10x breakpoint**: repeated wrapper replays dominate first; source-only edits remain cheap.

## Negative Tests

- **Malformed inputs**: missing top-level verifier file, stale `reference-backend/scripts/verify-production-proof-surface.sh` command strings, or old root calculations after the file move.
- **Error paths**: wrapper `require_file` checks still demand the old path, or the proof-page contract script still self-documents the deleted path.
- **Boundary conditions**: the public route stays `/docs/production-backend-proof/`, but the verifier command no longer depends on the retiring tree.

## Steps

1. Create `scripts/verify-production-proof-surface.sh` by moving the existing proof-page contract to the top-level `scripts/` directory, fixing its repo-root calculation, self-referenced command strings, and any artifact hints that still assume the old nested location.
2. Update `website/docs/docs/production-backend-proof/index.md` so its named public proof-page contract now points at `bash scripts/verify-production-proof-surface.sh` without changing the page’s public-secondary role.
3. Retarget the direct positive callers and verifier-contract assertions in `scripts/verify-m050-s01.sh`, `scripts/verify-m050-s03.sh`, `compiler/meshc/tests/e2e_m050_s01.rs`, and `compiler/meshc/tests/e2e_m050_s03.rs` so they require and archive the new top-level verifier path instead of the retiring repo-root copy.
4. Re-run the proof-page contract and the historical Rust contract tests so the move is green before any later task deletes `reference-backend/`.

## Must-Haves

- [ ] `scripts/verify-production-proof-surface.sh` becomes the canonical proof-page verifier path and fail-closes on the same public contract the old script enforced.
- [ ] `website/docs/docs/production-backend-proof/index.md` names the new verifier command instead of `bash reference-backend/scripts/verify-production-proof-surface.sh`.
- [ ] `scripts/verify-m050-s01.sh`, `scripts/verify-m050-s03.sh`, `compiler/meshc/tests/e2e_m050_s01.rs`, and `compiler/meshc/tests/e2e_m050_s03.rs` all point at the new path.
- [ ] No later task depends on the old nested verifier path remaining present.

## Verification

- `bash scripts/verify-production-proof-surface.sh`
- `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`
- `cargo test -p meshc --test e2e_m050_s03 -- --nocapture`

## Observability Impact

- Signals added/changed: the proof-page verifier’s own command banner, failing-file output, and artifact hints move to the top-level `scripts/` surface.
- How a future agent inspects this: run `bash scripts/verify-production-proof-surface.sh` directly, then inspect the failing wrapper log if `scripts/verify-m050-s01.sh` or `scripts/verify-m050-s03.sh` still points at the wrong path.
- Failure state exposed: missing top-level script, stale caller path, or root-calculation drift becomes visible immediately instead of failing later during deletion.

  - Estimate: 90m
  - Files: `scripts/verify-production-proof-surface.sh`, `website/docs/docs/production-backend-proof/index.md`, `scripts/verify-m050-s01.sh`, `scripts/verify-m050-s03.sh`, `compiler/meshc/tests/e2e_m050_s01.rs`, `compiler/meshc/tests/e2e_m050_s03.rs`
  - Verify: `bash scripts/verify-production-proof-surface.sh`
`cargo test -p meshc --test e2e_m050_s01 -- --nocapture`
`cargo test -p meshc --test e2e_m050_s03 -- --nocapture`
- [x] **T02: Removed the last public `reference-backend` doc wording and locked the docs contracts to the generic backend-proof handoff.** — ---
estimated_steps: 4
estimated_files: 7
skills_used:
  - vitepress
  - test
  - rust-testing
---

Clean up the remaining public wording that still leaks `reference-backend/` into the examples-first story, and strengthen the docs-side contract rails so those leaks cannot come back unnoticed.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| Public docs markdown | fail closed on the first stale `reference-backend` marker or missing Mesher/retained-verifier marker | N/A for source assertions | treat wrong wording or ordering as a real public-contract regression |
| Docs contract tests | keep each Node/Rust guard aligned with the shipped wording and fail on the first mismatch | use bounded test invocations; do not ignore red source contracts | treat stale exclusions or missing allowed markers as contract drift |
| S04 wrapper expectations | update the source and built-html checks together so `scripts/verify-m051-s04.sh` stays truthful | respect existing wrapper timeouts | treat built-html omissions or stale public markers as a release blocker |

## Load Profile

- **Shared resources**: public docs markdown, docs contract tests, and the `.tmp/m051-s04/verify/` built-html replay.
- **Per-operation cost**: a handful of markdown edits plus three contract-test updates and one S04 verifier contract update.
- **10x breakpoint**: repeated built-html replays and VitePress build checks dominate before source assertions do.

## Negative Tests

- **Malformed inputs**: bare `reference-backend/` mentions, same-file definition examples pinned to `reference-backend/api/jobs.mpl`, or distributed-proof bullets that still call the deleted app the deeper backend surface.
- **Error paths**: a source contract passes locally but `scripts/verify-m051-s04.sh` still expects the old wording in built HTML or copied wrapper output.
- **Boundary conditions**: public docs stay examples-first and generic while maintainer-only Mesher and retained-fixture surfaces remain discoverable only through the proof page.

## Steps

1. Rewrite the stale public wording in `website/docs/docs/tooling/index.md` and `website/docs/docs/distributed-proof/index.md` so the tooling proof is described generically against a backend-shaped project and the distributed proof page no longer names repo-root `reference-backend` as the deeper backend surface.
2. Tighten `scripts/tests/verify-m036-s03-contract.test.mjs`, `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`, and `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` so they fail closed on the exact stale markers this slice is removing.
3. Update `compiler/meshc/tests/e2e_m051_s04.rs` and `scripts/verify-m051-s04.sh` so the existing S04 acceptance surface expects the new wording and top-level proof-page verifier command instead of the deleted backend path.
4. Re-run the docs contracts and the S04 contract target to prove the public wording and built-html checks are aligned before the tree deletion task.

## Must-Haves

- [ ] `website/docs/docs/tooling/index.md` stops naming `reference-backend/` or `reference-backend/api/jobs.mpl` in the public LSP/editor proof story.
- [ ] `website/docs/docs/distributed-proof/index.md` removes the stale `reference-backend` deeper-backend bullet and keeps the public-secondary verifier map truthful.
- [ ] The M036, M050, and M051 docs/source contracts explicitly catch the removed stale markers.
- [ ] `scripts/verify-m051-s04.sh` and `compiler/meshc/tests/e2e_m051_s04.rs` stay green against the post-cleanup wording.

## Verification

- `node --test scripts/tests/verify-m036-s03-contract.test.mjs`
- `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`
- `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
- `cargo test -p meshc --test e2e_m051_s04 -- --nocapture`

## Observability Impact

- Signals added/changed: the existing S04 built-html contract and source-contract errors now name the last public `reference-backend` markers explicitly.
- How a future agent inspects this: start with the three Node contracts, then run `cargo test -p meshc --test e2e_m051_s04 -- --nocapture` if built-html or wrapper expectations still drift.
- Failure state exposed: the failing page, stale marker, or built-html omission is surfaced directly instead of being inferred from prose review.

  - Estimate: 2h
  - Files: `website/docs/docs/tooling/index.md`, `website/docs/docs/distributed-proof/index.md`, `scripts/tests/verify-m036-s03-contract.test.mjs`, `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`, `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`, `compiler/meshc/tests/e2e_m051_s04.rs`, `scripts/verify-m051-s04.sh`
  - Verify: `node --test scripts/tests/verify-m036-s03-contract.test.mjs`
`node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`
`node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
`cargo test -p meshc --test e2e_m051_s04 -- --nocapture`
- [x] **T03: Deleted the repo-root reference-backend tree and rewrote the retained S02 contracts to post-deletion truth.** — ---
estimated_steps: 5
estimated_files: 6
skills_used:
  - bash-scripting
  - rust-testing
  - test
---

Convert the retained backend fixture and its verifier from “compatibility copy still preserved” to “repo-root app is gone,” then delete the repo-root `reference-backend/` tree and its binary-ignore exception.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `scripts/verify-m051-s02.sh` | fail on the first missing retained fixture artifact or stale deleted-path expectation | fail closed at the current phase and keep the failing log and bundle hint | treat copied deleted-path artifacts or stale README commands as contract drift |
| Retained backend fixture docs/tests | fail on missing internal-runbook markers or stale compatibility-boundary language | N/A for source assertions | treat wrong retained-fixture commands or stale deletion assumptions as a real contract break |
| Filesystem delete step | stop immediately if the retained fixture path is targeted instead of only repo-root `reference-backend/` | N/A | treat any partial delete or surviving repo-root files as a blocker |

## Load Profile

- **Shared resources**: `.tmp/m051-s02/verify/`, the retained backend fixture tree, and DB-backed retained-backend replays.
- **Per-operation cost**: one retained README/test/verifier rewrite, one tree deletion, one `.gitignore` cleanup, and one DB-backed replay.
- **10x breakpoint**: the S02 assembled replay and its retained bundle copying dominate before source-only edits do.

## Negative Tests

- **Malformed inputs**: README or fixture tests still require `reference-backend/README.md`, S02 verifier still copies deleted compatibility files, or the deploy SQL comment still claims the deleted migration path.
- **Error paths**: `test ! -e reference-backend` is green but `scripts/verify-m051-s02.sh` still archives deleted-path artifacts or expects compatibility-boundary markers.
- **Boundary conditions**: the retained fixture remains authoritative and source-only after deletion, and only the repo-root compatibility copy disappears.

## Steps

1. Rewrite `scripts/fixtures/backend/reference-backend/README.md` and `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` to describe the retained fixture as the sole backend-only proof surface, removing the old “do not delete yet” compatibility boundary.
2. Update `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql`, `compiler/meshc/tests/e2e_m051_s02.rs`, and `scripts/verify-m051-s02.sh` so they stop requiring, copying, or documenting repo-root compatibility files and instead assert post-deletion truth.
3. Remove `reference-backend/` from the repo and drop the `reference-backend/reference-backend` ignore rule from `.gitignore`.
4. Verify the delete surface directly with `test ! -e reference-backend`, then re-run the S02 contract target and the DB-backed retained verifier so the retained backend-only proof is still green on the post-deletion tree.
5. Preserve the existing stale fixture-smoke worker cleanup and bundle-shape markers in `scripts/verify-m051-s02.sh`; deletion must not regress the retained backend replay’s debuggability.

## Must-Haves

- [ ] The retained fixture README and package tests no longer preserve repo-root compatibility files as a promised surface.
- [ ] `scripts/verify-m051-s02.sh` no longer copies or asserts `reference-backend/README.md` or the old nested proof-page verifier.
- [ ] `reference-backend/` is gone and `.gitignore` no longer hides a generated binary under that deleted tree.
- [ ] The retained backend-only contract still passes on the post-deletion tree.

## Verification

- `test ! -e reference-backend`
- `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s02.sh`

## Observability Impact

- Signals added/changed: the S02 verifier’s retained bundle and phase logs become post-deletion truth instead of compatibility-copy truth.
- How a future agent inspects this: check `test ! -e reference-backend`, then inspect `.tmp/m051-s02/verify/phase-report.txt`, `full-contract.log`, and the retained proof bundle pointer if the backend-only replay regresses.
- Failure state exposed: stale deleted-path assumptions, missing retained bundle markers, or accidental fixture deletion are surfaced as named S02 contract failures.

  - Estimate: 2h
  - Files: `scripts/fixtures/backend/reference-backend/README.md`, `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl`, `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql`, `compiler/meshc/tests/e2e_m051_s02.rs`, `scripts/verify-m051-s02.sh`, `.gitignore`
  - Verify: `test ! -e reference-backend`
`cargo test -p meshc --test e2e_m051_s02 -- --nocapture`
`DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s02.sh`
- [x] **T04: Added the M051 S05 post-deletion contract and wrapper, but the final replay still fails in the retained S02 fixture-smoke handoff.** — ---
estimated_steps: 4
estimated_files: 2
skills_used:
  - bash-scripting
  - rust-testing
  - test
---

Close the slice with one named post-deletion contract target and one assembled replay that composes the already-migrated M051 proof surfaces on the tree without `reference-backend/`.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `compiler/meshc/tests/e2e_m051_s05.rs` | fail on missing deleted-path assertions, missing delegated wrapper commands, or missing retained bundle markers | N/A for source assertions | treat stale callers or missing copied bundle markers as a real contract failure |
| `scripts/verify-m051-s05.sh` | stop on the first failing delegated phase and preserve the phase log plus artifact hint | record the timeout in `phase-report.txt` and fail closed | treat missing status files, pointer drift, or copied verify-tree drift as acceptance failure |
| Delegated S01–S04 wrappers | rely on each wrapper’s own phase markers and fail closed if any child replay regresses | preflight `DATABASE_URL` once and avoid hidden retries | treat a child wrapper that runs 0 tests or omits its bundle markers as a blocker |

## Load Profile

- **Shared resources**: `.tmp/m051-s01/verify/`, `.tmp/m051-s02/verify/`, `.tmp/m051-s03/verify/`, `.tmp/m051-s04/verify/`, and the new `.tmp/m051-s05/verify/` retained bundle.
- **Per-operation cost**: one Rust contract target, one assembled shell replay, and one full delegated post-deletion proof stack.
- **10x breakpoint**: the delegated wrapper stack and retained bundle copying dominate first; the new source contract itself is light.

## Negative Tests

- **Malformed inputs**: a surviving `reference-backend/` path, missing top-level proof-page verifier, or S05 replay that forgets to retain a delegated verify tree.
- **Error paths**: delegated S01–S04 wrappers stay green individually but S05 fails because the copied retained bundle markers or pointer file are wrong.
- **Boundary conditions**: the final post-deletion acceptance rail must compose Mesher, retained backend proof, tooling/editor rails, and the examples-first docs story together rather than proving only one subsystem.

## Steps

1. Add `compiler/meshc/tests/e2e_m051_s05.rs` as the slice-owned post-deletion contract target that asserts the repo-root tree is gone, the new top-level proof-page verifier exists, delegated wrapper commands are present, and the final retained bundle schema stays honest.
2. Add `scripts/verify-m051-s05.sh` as the authoritative final M051 replay: preflight `DATABASE_URL` once, run `bash scripts/verify-m051-s01.sh`, `bash scripts/verify-m051-s02.sh`, `bash scripts/verify-m051-s03.sh`, `bash scripts/verify-m051-s04.sh`, and publish `.tmp/m051-s05/verify/status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, and `latest-proof-bundle.txt`.
3. Copy the delegated S01–S04 verify trees and bundle pointers into `.tmp/m051-s05/verify/retained-proof-bundle/`, and make the S05 verifier fail closed if any delegated bundle is missing or malformed.
4. Re-run the new Rust contract target and the full `bash scripts/verify-m051-s05.sh` replay so the milestone closes on one stable post-deletion acceptance surface.

## Must-Haves

- [ ] `compiler/meshc/tests/e2e_m051_s05.rs` asserts the post-deletion contract instead of reusing prose-only checks.
- [ ] `bash scripts/verify-m051-s05.sh` is the authoritative post-deletion M051 replay and publishes the standard `.tmp/m051-s05/verify/` markers.
- [ ] The S05 retained bundle copies the delegated S01–S04 verifier state instead of depending on the deleted repo-root app path.
- [ ] Running the S05 replay is enough to prove the milestone goal on the post-deletion tree.

## Verification

- `cargo test -p meshc --test e2e_m051_s05 -- --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s05.sh`

## Observability Impact

- Signals added/changed: `.tmp/m051-s05/verify/status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, `latest-proof-bundle.txt`, and copied delegated verify trees become the final milestone-closeout inspection surface.
- How a future agent inspects this: start with `.tmp/m051-s05/verify/phase-report.txt`, then follow `latest-proof-bundle.txt` into the copied S01–S04 verify trees before re-running expensive delegated rails.
- Failure state exposed: the exact child phase, missing delegated artifact, or pointer drift is preserved under one stable post-deletion bundle.

  - Estimate: 90m
  - Files: `compiler/meshc/tests/e2e_m051_s05.rs`, `scripts/verify-m051-s05.sh`
  - Verify: `cargo test -p meshc --test e2e_m051_s05 -- --nocapture`
`DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s05.sh`
