---
estimated_steps: 4
estimated_files: 6
skills_used:
  - bash-scripting
  - vitepress
  - rust-testing
---

# T01: Move the proof-page verifier to a stable top-level path before deletion

**Slice:** S05 — Delete reference-backend and close the assembled acceptance rail
**Milestone:** M051

## Description

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

## Inputs

- `reference-backend/scripts/verify-production-proof-surface.sh` — existing proof-page verifier logic to relocate without widening scope
- `website/docs/docs/production-backend-proof/index.md` — current public proof-page command text
- `scripts/verify-m050-s01.sh` — wrapper that still requires the retiring verifier path
- `scripts/verify-m050-s03.sh` — wrapper that still requires the retiring verifier path
- `compiler/meshc/tests/e2e_m050_s01.rs` — historical contract assertions for the proof-page verifier path
- `compiler/meshc/tests/e2e_m050_s03.rs` — historical contract assertions for the proof-page verifier path

## Expected Output

- `scripts/verify-production-proof-surface.sh` — canonical post-deletion proof-page verifier
- `website/docs/docs/production-backend-proof/index.md` — public page updated to the new verifier path
- `scripts/verify-m050-s01.sh` — wrapper retargeted to the top-level verifier
- `scripts/verify-m050-s03.sh` — wrapper retargeted to the top-level verifier
- `compiler/meshc/tests/e2e_m050_s01.rs` — contract assertions updated to the new verifier path
- `compiler/meshc/tests/e2e_m050_s03.rs` — contract assertions updated to the new verifier path
