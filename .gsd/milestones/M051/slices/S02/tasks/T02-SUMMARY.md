---
id: T02
parent: S02
milestone: M051
provides: []
requires: []
affects: []
key_files: ["compiler/meshc/tests/support/m051_reference_backend.rs", "compiler/meshc/tests/support/mod.rs", "compiler/meshc/tests/e2e_reference_backend.rs", "compiler/meshc/tests/e2e_m051_s02.rs", ".gsd/KNOWLEDGE.md"]
key_decisions: ["D382: Build the retained backend runtime binary into `.tmp/m051-s02/reference-backend-runtime/`, but keep staged deploy bundles outside the repo root and publish only bundle pointers/manifests back under `.tmp/m051-s02/`.", "Rebind the legacy backend e2e target to the retained fixture through a shared support module instead of switching public repo-root compatibility scripts in this slice."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Verified the new slice-owned contract target with `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` (4 real tests passed), then replayed `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_stage_deploy_bundle -- --nocapture` to confirm the retained stage-deploy rail passed through the new support module. Replayed `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` and `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql"` to confirm the T02 cutover did not break the retained fixture’s source-only package tests or staging flow. Ran `bash scripts/verify-m051-s02.sh` as the slice-level visibility check; it failed with `No such file or directory`, which is expected until T03 lands the assembled verifier."
completed_at: 2026-04-04T08:50:41.170Z
blocker_discovered: false
---

# T02: Rebound the retained backend e2e rails to the internal fixture and added M051/S02 contract tests.

> Rebound the retained backend e2e rails to the internal fixture and added M051/S02 contract tests.

## What Happened
---
id: T02
parent: S02
milestone: M051
key_files:
  - compiler/meshc/tests/support/m051_reference_backend.rs
  - compiler/meshc/tests/support/mod.rs
  - compiler/meshc/tests/e2e_reference_backend.rs
  - compiler/meshc/tests/e2e_m051_s02.rs
  - .gsd/KNOWLEDGE.md
key_decisions:
  - D382: Build the retained backend runtime binary into `.tmp/m051-s02/reference-backend-runtime/`, but keep staged deploy bundles outside the repo root and publish only bundle pointers/manifests back under `.tmp/m051-s02/`.
  - Rebind the legacy backend e2e target to the retained fixture through a shared support module instead of switching public repo-root compatibility scripts in this slice.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T08:50:41.170Z
blocker_discovered: false
---

# T02: Rebound the retained backend e2e rails to the internal fixture and added M051/S02 contract tests.

**Rebound the retained backend e2e rails to the internal fixture and added M051/S02 contract tests.**

## What Happened

Added `compiler/meshc/tests/support/m051_reference_backend.rs` as the shared retained-backend harness surface for the internal fixture under `scripts/fixtures/backend/reference-backend`. The module now owns the canonical retained fixture path, the stable runtime build artifact path under `.tmp/m051-s02/reference-backend-runtime/`, the retained `meshc migrate` invocation, the retained smoke/stage script entrypoints, and bundle-pointer/manifest helpers. Then I rewired `compiler/meshc/tests/e2e_reference_backend.rs` to use that module for retained build, migration, smoke, and stage-deploy seams so the named stage-deploy rail no longer shells through the repo-root compatibility copy or depends on `reference-backend/reference-backend`. Finally, I added `compiler/meshc/tests/e2e_m051_s02.rs` as the slice-owned contract rail for retained fixture path resolution, staged bundle shape, source-only drift, and support-module delegation, and recorded the non-obvious bundle-location rule in `.gsd/KNOWLEDGE.md`.

## Verification

Verified the new slice-owned contract target with `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` (4 real tests passed), then replayed `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_stage_deploy_bundle -- --nocapture` to confirm the retained stage-deploy rail passed through the new support module. Replayed `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` and `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql"` to confirm the T02 cutover did not break the retained fixture’s source-only package tests or staging flow. Ran `bash scripts/verify-m051-s02.sh` as the slice-level visibility check; it failed with `No such file or directory`, which is expected until T03 lands the assembled verifier.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` | 0 | ✅ pass | 43800ms |
| 2 | `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_stage_deploy_bundle -- --nocapture` | 0 | ✅ pass | 50500ms |
| 3 | `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` | 0 | ✅ pass | 30600ms |
| 4 | `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql"` | 0 | ✅ pass | 6190ms |
| 5 | `bash scripts/verify-m051-s02.sh` | 127 | ❌ fail | 12ms |


## Deviations

I did not move every HTTP/DB/process/recovery helper out of `e2e_reference_backend.rs` in this task. The extraction focused on the retained fixture path/build/migrate/stage seams that the T02 acceptance rails actually exercise, while the deeper runtime helper bodies remain local for now.

## Known Issues

`scripts/verify-m051-s02.sh` is still missing, so the slice-level assembled verifier remains red until T03 implements the fail-closed replay wrapper.

## Files Created/Modified

- `compiler/meshc/tests/support/m051_reference_backend.rs`
- `compiler/meshc/tests/support/mod.rs`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `compiler/meshc/tests/e2e_m051_s02.rs`
- `.gsd/KNOWLEDGE.md`


## Deviations
I did not move every HTTP/DB/process/recovery helper out of `e2e_reference_backend.rs` in this task. The extraction focused on the retained fixture path/build/migrate/stage seams that the T02 acceptance rails actually exercise, while the deeper runtime helper bodies remain local for now.

## Known Issues
`scripts/verify-m051-s02.sh` is still missing, so the slice-level assembled verifier remains red until T03 implements the fail-closed replay wrapper.
