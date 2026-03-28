---
id: T01
parent: S01
milestone: M038
provides: []
requires: []
affects: []
key_files: ["compiler/mesh-codegen/src/lib.rs", ".github/workflows/release.yml"]
key_decisions: ["Removed vcpkg libxml2 entirely rather than fixing the LIB path — the clang+llvm tarball bundles its own libxml2s.lib and llvm-sys already discovers it via llvm-config --libdir"]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "cargo test -p mesh-codegen link -- --nocapture: 8 passed. rg 'FORCE:MULTIPLE' .github/workflows/ returned no matches. YAML validation passed. pre-llvm-init stage confirmed present in both binary entry points."
completed_at: 2026-03-28T01:40:01.540Z
blocker_discovered: false
---

# T01: Added pre-LLVM-init build trace stage and removed vcpkg libxml2 + FORCE:MULTIPLE from Windows CI build

> Added pre-LLVM-init build trace stage and removed vcpkg libxml2 + FORCE:MULTIPLE from Windows CI build

## What Happened
---
id: T01
parent: S01
milestone: M038
key_files:
  - compiler/mesh-codegen/src/lib.rs
  - .github/workflows/release.yml
key_decisions:
  - Removed vcpkg libxml2 entirely rather than fixing the LIB path — the clang+llvm tarball bundles its own libxml2s.lib and llvm-sys already discovers it via llvm-config --libdir
duration: ""
verification_result: passed
completed_at: 2026-03-28T01:40:01.541Z
blocker_discovered: false
---

# T01: Added pre-LLVM-init build trace stage and removed vcpkg libxml2 + FORCE:MULTIPLE from Windows CI build

**Added pre-LLVM-init build trace stage and removed vcpkg libxml2 + FORCE:MULTIPLE from Windows CI build**

## What Happened

Two changes: (1) Added build_trace::set_stage("pre-llvm-init") before Context::create() in both compile_to_binary and compile_mir_to_binary for better Windows crash diagnostics. (2) Removed three items from release.yml: the vcpkg libxml2 install step, the LIB env var override, and the RUSTFLAGS with /FORCE:MULTIPLE. The duplicate symbol root cause was both LLVM's bundled libxml2s.lib and the vcpkg copy being on the linker path. The llvm-sys crate already adds the LLVM tarball's lib directory via llvm-config --libdir, so removing the vcpkg copy eliminates the conflict.

## Verification

cargo test -p mesh-codegen link -- --nocapture: 8 passed. rg 'FORCE:MULTIPLE' .github/workflows/ returned no matches. YAML validation passed. pre-llvm-init stage confirmed present in both binary entry points.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p mesh-codegen link -- --nocapture` | 0 | ✅ pass | 15300ms |
| 2 | `! rg -q 'FORCE:MULTIPLE' .github/workflows/release.yml` | 0 | ✅ pass | 100ms |
| 3 | `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` | 0 | ✅ pass | 100ms |


## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `compiler/mesh-codegen/src/lib.rs`
- `.github/workflows/release.yml`


## Deviations
None.

## Known Issues
None.
