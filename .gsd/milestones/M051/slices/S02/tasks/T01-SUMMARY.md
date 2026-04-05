---
id: T01
parent: S02
milestone: M051
provides: []
requires: []
affects: []
key_files: ["scripts/fixtures/backend/reference-backend/mesh.toml", "scripts/fixtures/backend/reference-backend/main.mpl", "scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql", "scripts/fixtures/backend/reference-backend/README.md", "scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh", "scripts/fixtures/backend/reference-backend/scripts/smoke.sh", "scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl", ".gsd/KNOWLEDGE.md"]
key_decisions: ["Keep the retained fixture source-only by building it with explicit `--output` artifact paths and leave the repo-root public proof script on the compatibility copy instead of duplicating it here."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Task-local verification passed: shell syntax checks for the new fixture scripts succeeded; `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` passed with both the copied config tests and the new fixture-contract tests; `bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$(mktemp -d)"` staged a runnable bundle and left `scripts/fixtures/backend/reference-backend/reference-backend` absent; and a negative probe confirmed the stage script fails immediately on missing deploy SQL instead of drifting to the repo-root package. Slice-level visibility checks were also run and are still red by plan: `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` fails because the target does not exist yet (owned by T02), and `bash scripts/verify-m051-s02.sh` fails because the assembled verifier does not exist yet (owned by T03)."
completed_at: 2026-04-04T08:36:48.946Z
blocker_discovered: false
---

# T01: Materialized the internal `reference-backend` fixture under `scripts/fixtures/backend/` with artifact-local stage/smoke scripts and fixture-local contract tests.

> Materialized the internal `reference-backend` fixture under `scripts/fixtures/backend/` with artifact-local stage/smoke scripts and fixture-local contract tests.

## What Happened
---
id: T01
parent: S02
milestone: M051
key_files:
  - scripts/fixtures/backend/reference-backend/mesh.toml
  - scripts/fixtures/backend/reference-backend/main.mpl
  - scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql
  - scripts/fixtures/backend/reference-backend/README.md
  - scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh
  - scripts/fixtures/backend/reference-backend/scripts/smoke.sh
  - scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Keep the retained fixture source-only by building it with explicit `--output` artifact paths and leave the repo-root public proof script on the compatibility copy instead of duplicating it here.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T08:36:48.948Z
blocker_discovered: false
---

# T01: Materialized the internal `reference-backend` fixture under `scripts/fixtures/backend/` with artifact-local stage/smoke scripts and fixture-local contract tests.

**Materialized the internal `reference-backend` fixture under `scripts/fixtures/backend/` with artifact-local stage/smoke scripts and fixture-local contract tests.**

## What Happened

Copied the backend package source, tests, migrations, deploy SQL, and runtime/deploy scripts from repo-root `reference-backend/` into `scripts/fixtures/backend/reference-backend/` while deliberately leaving the tracked repo-root binary and public proof-surface script on the compatibility path. Added a maintainer-only fixture README, rewrote the fixture-local `scripts/stage-deploy.sh` and `scripts/smoke.sh` to build with explicit `--output` artifact paths instead of writing an in-place binary, and added `tests/fixture.test.mpl` so the relocation fails closed on manifest/path/script drift. Repo-root `reference-backend/` remains intact for later retarget/delete slices.

## Verification

Task-local verification passed: shell syntax checks for the new fixture scripts succeeded; `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` passed with both the copied config tests and the new fixture-contract tests; `bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$(mktemp -d)"` staged a runnable bundle and left `scripts/fixtures/backend/reference-backend/reference-backend` absent; and a negative probe confirmed the stage script fails immediately on missing deploy SQL instead of drifting to the repo-root package. Slice-level visibility checks were also run and are still red by plan: `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` fails because the target does not exist yet (owned by T02), and `bash scripts/verify-m051-s02.sh` fails because the assembled verifier does not exist yet (owned by T03).

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `bash -n scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh && bash -n scripts/fixtures/backend/reference-backend/scripts/smoke.sh` | 0 | ✅ pass | 68ms |
| 2 | `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` | 0 | ✅ pass | 9383ms |
| 3 | `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql" && test ! -e scripts/fixtures/backend/reference-backend/reference-backend` | 0 | ✅ pass | 7030ms |
| 4 | `temporarily remove scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql and run bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$(mktemp -d)" expecting a missing-artifact failure` | 0 | ✅ pass | 169ms |
| 5 | `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` | 101 | ❌ fail | 521ms |
| 6 | `bash scripts/verify-m051-s02.sh` | 127 | ❌ fail | 87ms |


## Deviations

Added `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` and updated the copied fixture-local `scripts/smoke.sh` in T01 instead of waiting for later slice tasks so the relocation proves the new source-only build contract immediately.

## Known Issues

The slice-owned Rust target `compiler/meshc/tests/e2e_m051_s02.rs` and assembled verifier `scripts/verify-m051-s02.sh` do not exist yet, so the slice-level rails remain red until T02 and T03 land. No additional blocker was discovered inside the T01 scope.

## Files Created/Modified

- `scripts/fixtures/backend/reference-backend/mesh.toml`
- `scripts/fixtures/backend/reference-backend/main.mpl`
- `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql`
- `scripts/fixtures/backend/reference-backend/README.md`
- `scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh`
- `scripts/fixtures/backend/reference-backend/scripts/smoke.sh`
- `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl`
- `.gsd/KNOWLEDGE.md`


## Deviations
Added `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl` and updated the copied fixture-local `scripts/smoke.sh` in T01 instead of waiting for later slice tasks so the relocation proves the new source-only build contract immediately.

## Known Issues
The slice-owned Rust target `compiler/meshc/tests/e2e_m051_s02.rs` and assembled verifier `scripts/verify-m051-s02.sh` do not exist yet, so the slice-level rails remain red until T02 and T03 land. No additional blocker was discovered inside the T01 scope.
