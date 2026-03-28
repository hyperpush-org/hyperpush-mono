---
depends_on: [M034]
---

# M038: Windows Release Smoke Fix — Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Problem

The hosted `release.yml` workflow fails on `v0.1.0` at the `Verify release assets (x86_64-pc-windows-msvc)` job. The failure is an access violation (exit code `-1073741819` / `STATUS_ACCESS_VIOLATION`) in the installed `meshc.exe build` step when compiling the trivial `scripts/fixtures/m034-s03-installer-smoke/` fixture (`println("hello")`).

This is the sole remaining red lane — five of six hosted workflow lanes are green on the rollout SHA (`1e83ea930fdfd346b9e56659dc50d2f759ec5da2`).

## Evidence

- Hosted run: `https://github.com/snowdamiz/mesh-lang/actions/runs/23669185030`
- Failed job: `Verify release assets (x86_64-pc-windows-msvc)` at job `68969102166`
- Diagnostic artifact: `release-smoke-x86_64-pc-windows-msvc-diagnostics`
- Local evidence: `.tmp/m034-s12/t01/diagnostic-summary.json` — classification `pre-object`, no build trace written
- The crash happens before `MESH_BUILD_TRACE_PATH` is written, so the failure is in the compiler pipeline before LLVM object emission

## What's Already In Place

- `compiler/mesh-codegen/src/link.rs` has full target-aware link planning with Windows MSVC support (`mesh_rt.lib`, `clang.exe` driver, `LLVM_SYS_211_PREFIX` resolution)
- `compiler/mesh-codegen/src/lib.rs` has structured `build_trace` infrastructure for classifying failure phases
- `scripts/verify-m034-s03.ps1` has `Push-InstalledBuildEnvironment` that exports `CARGO_TARGET_DIR` before the installed build
- The CI workflow already installs Rust, LLVM 21, builds `mesh-rt`, and sets `LLVM_SYS_211_PREFIX` before the smoke step
- The fixture is minimal: `fn main() do println("hello") end` with no dependencies

## Scope

1. Diagnose the root cause of the Windows `meshc.exe` access violation on the installed-compiler path
2. Fix the compiler/codegen/linker pipeline so `meshc.exe build` works on Windows MSVC
3. Verify locally via `cargo test -p mesh-codegen link -- --nocapture` and the PowerShell verifier pattern
4. Get the hosted `release.yml` green on the approved `v0.1.0` tag

## Success Criteria

- `pwsh -NoProfile -File scripts/verify-m034-s03.ps1` passes in CI on `windows-latest`
- The `release.yml` `Verify release assets (x86_64-pc-windows-msvc)` job goes green
- `scripts/verify-m034-s05.sh` full assembly replay passes (all six lanes green)

## What This Does NOT Cover

- Broad Windows platform support beyond the installed-compiler smoke path
- New Windows-specific features or test coverage
- Changes to the release asset bundle format or installer UX

## Existing Codebase To Revisit

- `compiler/mesh-codegen/src/link.rs` — link planning and target-aware linker selection
- `compiler/mesh-codegen/src/lib.rs` — `compile_to_binary` / `compile_to_binary_from_source` entry points and build trace
- `compiler/meshc/tests/e2e_m034_s12.rs` — existing Windows smoke regression tests
- `scripts/verify-m034-s03.ps1` — the canonical PowerShell verifier
- `.github/workflows/release.yml` — the hosted release workflow (lines 405-460)
- `.tmp/m034-s12/` and `.tmp/m034-s11/t03/diag-download/` — prior diagnostic artifacts
