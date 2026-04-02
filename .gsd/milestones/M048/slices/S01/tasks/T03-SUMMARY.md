---
id: T03
parent: S01
milestone: M048
provides: []
requires: []
affects: []
key_files: ["compiler/meshc/tests/e2e_m048_s01.rs", "compiler/mesh-codegen/src/lib.rs", ".gsd/KNOWLEDGE.md", ".gsd/DECISIONS.md"]
key_decisions: ["Merge MIR modules starting with the designated entry module so duplicate `mesh_main` symbols honor the resolved executable entrypoint instead of source-order.", "Retain per-scenario project snapshots and subprocess stdout/stderr under `.tmp/m048-s01/` so the first broken build/test seam is diagnosable without rerunning under ad hoc instrumentation."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Verified the new codegen seam with `cargo test -p mesh-codegen merge_mir_modules_prefers_entry_module_mesh_main_when_multiple_modules_define_main -- --nocapture`, then reran the dedicated acceptance rail with `cargo test -p meshc --test e2e_m048_s01 -- --nocapture`. After that, I reran the full slice closeout commands from T01, T02, and T03 — `cargo test -p mesh-pkg entrypoint -- --nocapture`, `cargo test -p meshc build_project_ -- --nocapture`, `cargo test -p meshc --test tooling_e2e test_test_ -- --nocapture`, and `cargo test -p meshc --test e2e_m048_s01 -- --nocapture` — and all passed."
completed_at: 2026-04-02T07:28:08.406Z
blocker_discovered: false
---

# T03: Added the M048/S01 acceptance rail and fixed MIR merge so manifest-selected entrypoints win end-to-end.

> Added the M048/S01 acceptance rail and fixed MIR merge so manifest-selected entrypoints win end-to-end.

## What Happened
---
id: T03
parent: S01
milestone: M048
key_files:
  - compiler/meshc/tests/e2e_m048_s01.rs
  - compiler/mesh-codegen/src/lib.rs
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
key_decisions:
  - Merge MIR modules starting with the designated entry module so duplicate `mesh_main` symbols honor the resolved executable entrypoint instead of source-order.
  - Retain per-scenario project snapshots and subprocess stdout/stderr under `.tmp/m048-s01/` so the first broken build/test seam is diagnosable without rerunning under ad hoc instrumentation.
duration: ""
verification_result: passed
completed_at: 2026-04-02T07:28:08.406Z
blocker_discovered: false
---

# T03: Added the M048/S01 acceptance rail and fixed MIR merge so manifest-selected entrypoints win end-to-end.

**Added the M048/S01 acceptance rail and fixed MIR merge so manifest-selected entrypoints win end-to-end.**

## What Happened

Created `compiler/meshc/tests/e2e_m048_s01.rs` as the dedicated slice acceptance target. The new harness writes isolated temp projects for a default root-`main.mpl` control, an override-entry project where both root and override entry files exist, an override-only build without a root `main.mpl`, and an override-entry test project exercised through project-dir, tests-dir, and specific-file `meshc test` targets. The harness fails closed on malformed fixture paths, missing resolved entry files, and invalid retained artifact state; it wraps `meshc build`, `meshc test`, and compiled-binary execution with explicit timeouts; and it archives the fixture tree plus per-command stdout/stderr under `.tmp/m048-s01/` so wrong-project, zero-proof, or timeout failures stay inspectable. While bringing the rail green, it exposed a real compiler bug outside the planned file list: `meshc build` still executed the legacy root `main.mpl` when both entry files existed because duplicate `mesh_main` MIR symbols were merged in source order. I fixed `compiler/mesh-codegen/src/lib.rs::merge_mir_modules(...)` so the designated entry module wins that collision, added a focused `mesh-codegen` regression, and recorded the resulting codegen/diagnostic knowledge for downstream work.

## Verification

Verified the new codegen seam with `cargo test -p mesh-codegen merge_mir_modules_prefers_entry_module_mesh_main_when_multiple_modules_define_main -- --nocapture`, then reran the dedicated acceptance rail with `cargo test -p meshc --test e2e_m048_s01 -- --nocapture`. After that, I reran the full slice closeout commands from T01, T02, and T03 — `cargo test -p mesh-pkg entrypoint -- --nocapture`, `cargo test -p meshc build_project_ -- --nocapture`, `cargo test -p meshc --test tooling_e2e test_test_ -- --nocapture`, and `cargo test -p meshc --test e2e_m048_s01 -- --nocapture` — and all passed.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p mesh-codegen merge_mir_modules_prefers_entry_module_mesh_main_when_multiple_modules_define_main -- --nocapture` | 0 | ✅ pass | 50ms |
| 2 | `cargo test -p mesh-pkg entrypoint -- --nocapture` | 0 | ✅ pass | 50ms |
| 3 | `cargo test -p meshc build_project_ -- --nocapture` | 0 | ✅ pass | 10ms |
| 4 | `cargo test -p meshc --test tooling_e2e test_test_ -- --nocapture` | 0 | ✅ pass | 9510ms |
| 5 | `cargo test -p meshc --test e2e_m048_s01 -- --nocapture` | 0 | ✅ pass | 9950ms |


## Deviations

Updated `compiler/mesh-codegen/src/lib.rs` and added a focused unit regression there outside the original task file list because the new acceptance rail surfaced a real duplicate-`mesh_main` merge bug that prevented manifest-selected build entrypoints from winning when both entry files existed.

## Known Issues

None.

## Files Created/Modified

- `compiler/meshc/tests/e2e_m048_s01.rs`
- `compiler/mesh-codegen/src/lib.rs`
- `.gsd/KNOWLEDGE.md`
- `.gsd/DECISIONS.md`


## Deviations
Updated `compiler/mesh-codegen/src/lib.rs` and added a focused unit regression there outside the original task file list because the new acceptance rail surfaced a real duplicate-`mesh_main` merge bug that prevented manifest-selected build entrypoints from winning when both entry files existed.

## Known Issues
None.
