---
estimated_steps: 5
estimated_files: 6
skills_used:
  - bash-scripting
  - rust-testing
  - test
---

# T03: Flip the retained S02 contract to post-deletion truth and remove the repo-root tree

**Slice:** S05 — Delete reference-backend and close the assembled acceptance rail
**Milestone:** M051

## Description

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

## Inputs

- `scripts/fixtures/backend/reference-backend/README.md` — retained runbook that still preserves the compatibility boundary
- `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` — retained package test that still expects compatibility markers
- `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql` — deploy SQL comment that still points at the repo-root app path
- `compiler/meshc/tests/e2e_m051_s02.rs` — S02 contract target that still encodes deleted compatibility files
- `scripts/verify-m051-s02.sh` — retained backend verifier that still copies deleted compatibility artifacts
- `reference-backend/README.md` — repo-root compatibility file scheduled for deletion
- `reference-backend/scripts/verify-production-proof-surface.sh` — repo-root compatibility verifier scheduled for deletion
- `.gitignore` — legacy binary ignore rule for the retiring tree

## Expected Output

- `scripts/fixtures/backend/reference-backend/README.md` — retained runbook rewritten to post-deletion truth
- `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` — retained package test aligned to post-deletion truth
- `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql` — deploy SQL comment corrected to retained-fixture truth
- `compiler/meshc/tests/e2e_m051_s02.rs` — retained backend contract updated to post-deletion expectations
- `scripts/verify-m051-s02.sh` — retained backend verifier updated to post-deletion bundle and contract checks
- `.gitignore` — legacy repo-root binary ignore removed
