# M038 Research: Windows Release Smoke Fix

## Problem Summary

The hosted `release.yml` workflow on `v0.1.0` fails at the `Verify release assets (x86_64-pc-windows-msvc)` job. The installed `meshc.exe build` crashes with exit code `-1073741819` (`STATUS_ACCESS_VIOLATION`) when compiling a trivial `println("hello")` fixture. This is the only remaining red lane — five of six hosted workflow lanes are green.

## Evidence Analysis

### Crash Characteristics

- `meshc.exe --version` succeeds (exit 0) — the binary loads and runs Rust code
- `meshc.exe build <dir>` crashes immediately with `STATUS_ACCESS_VIOLATION`
- Empty stdout and stderr — no Rust panic output, no build trace written
- The `MESH_BUILD_TRACE_PATH` env var is set but the trace file is never created
- Diagnostic classification: `pre-object` (crash before LLVM object emission)

### Why No Trace Was Written

The `build_trace::set_compile_context()` call at the top of `compile_mir_to_binary()` writes the trace file. Since no trace exists, the crash occurs either:
1. Before `compile_mir_to_binary` is called (in parse/typecheck/MIR lowering — all pure Rust, unlikely)
2. Inside `compile_mir_to_binary` but before `set_compile_context` finishes (unlikely — it's just JSON I/O)
3. During `Context::create()` or `Target::initialize_native()` — the first LLVM FFI calls

Actually, examining the code flow more carefully: `compile_mir_to_binary` calls `set_compile_context` first, then `prepare_link`, then enters the inner closure that calls `Context::create()` and `CodeGen::new()`. The trace SHOULD be written before any LLVM call. If it's not written, the crash may be in the pure-Rust discovery/parse/typecheck/MIR pipeline that runs in `main.rs::build()` BEFORE `compile_mir_to_binary`. But those are pure Rust with no FFI.

The most likely explanation: the crash IS in `compile_mir_to_binary` at the LLVM initialization, and the trace write itself silently failed (e.g., the trace path directory didn't exist or had a Windows permission issue), OR the access violation happens during program-level static initialization of LLVM globals on the first code path that touches LLVM symbols.

### Root Cause: `/FORCE:MULTIPLE` During `cargo build`

The Windows build step in `release.yml` uses:
```yaml
RUSTFLAGS: -Clink-args=/FORCE:MULTIPLE
```

This flag instructs the MSVC linker to accept multiply-defined symbols instead of erroring. It was added to work around duplicate symbol errors when statically linking LLVM 21 libraries on Windows.

**Why this causes the access violation:**
- On Windows MSVC, LLVM's COMDAT sections use `IMAGE_COMDAT_SELECT_NODUPLICATES`, causing the linker to reject functions with multiple definitions by default
- `/FORCE:MULTIPLE` suppresses these errors but lets the linker arbitrarily pick which copy of a multiply-defined symbol to use
- If the wrong copy is picked (e.g., a stub function instead of the real implementation), calling it at runtime causes an access violation
- `--version` works because it never touches LLVM code paths
- `build` crashes because it calls `Target::initialize_native()` which invokes LLVM C API functions that may have been resolved to the wrong symbol copy

### Library Dependencies Contributing to Duplicate Symbols

The duplicate symbols likely come from the intersection of:
1. **llvm-sys 211.0.0** — statically links the LLVM 21 libraries (prefer-static default)
2. **libxml2** — installed via vcpkg as `libxml2:x64-windows-static` with a manual copy from `libxml2.lib` to `libxml2s.lib`
3. **mesh-rt** — a `staticlib` that includes many C-level dependencies (libsqlite3-sys bundled, rustls, etc.)

The `cargo build --release` step compiles the entire workspace. When LLVM static libs AND other static libs provide overlapping C runtime symbols, the MSVC linker normally rejects them. `/FORCE:MULTIPLE` papers over this but introduces the runtime crash.

## What Exists in the Codebase

### Compiler Pipeline (`compiler/mesh-codegen/`)
- `src/lib.rs` — `compile_mir_to_binary` with build trace infrastructure, `compile_to_binary` for single-module
- `src/codegen/mod.rs` — `CodeGen::new()` with `Target::initialize_native()`, `compile()`, `emit_object()`
- `src/link.rs` — Full target-aware link planning with Windows MSVC support (`mesh_rt.lib`, `clang.exe` driver)
- Build trace records: `lastStage`, `objectEmissionStarted/Completed`, `runtimeLibraryPath`, `linkerProgram`

### CI Workflow (`.github/workflows/release.yml`)
- Windows LLVM 21 install: `clang+llvm-21.1.8-x86_64-pc-windows-msvc.tar.xz` extracted to `~/llvm`
- `LLVM_SYS_211_PREFIX` pointed at the extracted LLVM
- `libxml2:x64-windows-static` via vcpkg with `libxml2.lib` → `libxml2s.lib` copy
- Build: `cargo build --release --target x86_64-pc-windows-msvc` with `RUSTFLAGS: -Clink-args=/FORCE:MULTIPLE`

### Smoke Verifier (`scripts/verify-m034-s03.ps1`)
- Builds tooling with `cargo build -q -p meshc -p meshpkg`
- Installs from staged archives, runs `meshc.exe --version`, then `meshc.exe build <fixture>`
- `Push-InstalledBuildEnvironment` sets `CARGO_TARGET_DIR` to repo-root `target/`
- `Invoke-InstalledBuildCommand` sets `MESH_BUILD_TRACE_PATH` and captures diagnostic summary

### Existing Tests (`compiler/meshc/tests/e2e_m034_s12.rs`)
- `m034_s12_native_build_trace_records_object_and_link_context` — build trace writes all fields
- `m034_s12_missing_runtime_lookup_is_reported_before_object_emission` — missing runtime fails gracefully
- `m034_s12_bad_windows_llvm_prefix_is_reported_before_object_emission` — bad LLVM prefix fails gracefully

## Fix Strategy

### Primary Approach: Remove `/FORCE:MULTIPLE` and Fix the Underlying Duplicate Symbol Issue

The clean fix is to eliminate the need for `/FORCE:MULTIPLE` rather than working around its consequences:

1. **Identify the specific duplicate symbols** by building without `/FORCE:MULTIPLE` and reading the linker errors
2. **Fix the duplication source** — likely one of:
   - Use `prefer-dynamic` or configure llvm-sys to avoid static lib conflicts
   - Adjust the vcpkg libxml2 install to not conflict with LLVM's own libxml2
   - Use LLVM's shared libraries on Windows instead of static
   - Build LLVM from source with the right CMake options to avoid the conflict
   - Use `/FORCE:UNRESOLVED` (more targeted) instead of `/FORCE:MULTIPLE` (blanket)

### Fallback Approach: If Duplicate Symbols Can't Be Eliminated

If the duplicate symbols are inherent to the LLVM+Rust toolchain combination:
1. Use `/FORCE:UNRESOLVED` + `/OPT:REF` instead of `/FORCE:MULTIPLE` — this performs dead-code elimination before symbol resolution, which is the safer alternative
2. Or: separate the LLVM-dependent code into a DLL that's dynamically loaded, isolating the symbol namespace

### Validation Approach

The fix must be validated through:
1. Local: `cargo build --release --target x86_64-pc-windows-msvc` succeeds without `/FORCE:MULTIPLE` (or with the safer alternative)
2. Local: `cargo test -p mesh-codegen link -- --nocapture` passes
3. Hosted: The `release.yml` `Verify release assets (x86_64-pc-windows-msvc)` job goes green
4. Assembly: `scripts/verify-m034-s05.sh` full replay passes (all six lanes green)

**Challenge: no Windows CI runner is available locally.** The diagnosis and fix development must happen through:
- Reading the LLVM/linker diagnostic output from CI runs
- Making targeted changes and pushing to trigger hosted workflows
- Using the structured build trace and diagnostic summary infrastructure

## Constraints and Risks

### High Risk: Diagnosis Without Local Windows
The biggest risk is the feedback loop. Each hypothesis requires a CI push-and-wait cycle. The milestone should be structured to minimize round-trips by:
- Maximizing diagnostic output in the first push
- Having a clear hypothesis chain where each CI run either confirms the fix or provides enough data for the next attempt

### Medium Risk: LLVM Version Sensitivity
The LLVM 21 static library layout on Windows may have version-specific quirks. The fix should be tested against the exact same LLVM version (21.1.8) used in CI.

### Low Risk: Collateral Damage to Other Platforms
Any fix must not break the five green lanes. The CI matrix already tests all platforms, so this is structurally protected.

### Constraint: Existing Verifier Contract
The `scripts/verify-m034-s03.ps1` verifier is the authoritative local gate. Changes should flow through the existing verifier rather than adding a parallel verification path.

## Natural Slice Boundaries

### Slice 1: Diagnose and Fix Windows Build
- Remove `/FORCE:MULTIPLE`, capture the actual linker errors
- Fix the underlying duplicate symbol issue (likely libxml2 or LLVM static lib configuration)
- Add diagnostic breadcrumbs (e.g., a pre-LLVM-init trace marker in `compile_mir_to_binary`)
- Push to hosted CI and verify

### Slice 2: Hosted Green Closeout
- Confirm `release.yml` Windows job goes green on the tag
- Run full `scripts/verify-m034-s05.sh` assembly replay
- Archive `first-green` evidence

This is a two-slice milestone where S01 contains all the technical work and S02 is the hosted verification and evidence collection.

## What Should Be Proven First

S01 should start by removing `/FORCE:MULTIPLE` and reading the resulting linker errors. Those errors are the actual diagnostic — they tell us exactly which symbols are duplicated and from which libraries. Everything else is hypothesis until we see those errors.

## Existing Patterns to Reuse

- Build trace infrastructure (`MESH_BUILD_TRACE_PATH`) for classifying failure phases
- `scripts/verify-m034-s03.ps1` and `Invoke-InstalledBuildCommand` for structured smoke verification
- `scripts/verify-m034-s06-remote-evidence.sh` for hosted evidence collection
- `scripts/verify-m034-s05.sh` for full assembly replay
- The staged fast-forward push pattern if the full-range push times out again

## Skills Discovered

No new skills needed. This is a Rust + LLVM + Windows MSVC linking problem within the existing codebase.
