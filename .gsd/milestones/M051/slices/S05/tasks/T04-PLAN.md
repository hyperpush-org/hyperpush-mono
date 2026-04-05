---
estimated_steps: 4
estimated_files: 2
skills_used:
  - bash-scripting
  - rust-testing
  - test
---

# T04: Add the final S05 contract and assembled post-deletion acceptance rail

**Slice:** S05 — Delete reference-backend and close the assembled acceptance rail
**Milestone:** M051

## Description

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

## Inputs

- `scripts/verify-m051-s01.sh` — maintained Mesher replay consumed by the final assembly rail
- `scripts/verify-m051-s02.sh` — retained backend-only replay consumed by the final assembly rail
- `scripts/verify-m051-s03.sh` — tooling/editor replay consumed by the final assembly rail
- `scripts/verify-m051-s04.sh` — docs/scaffold/skill replay consumed by the final assembly rail
- `scripts/verify-production-proof-surface.sh` — top-level public proof-page verifier that must exist after deletion

## Expected Output

- `compiler/meshc/tests/e2e_m051_s05.rs` — slice-owned post-deletion contract target
- `scripts/verify-m051-s05.sh` — authoritative assembled post-deletion replay
