---
estimated_steps: 13
estimated_files: 2
skills_used: []
---

# T01: Add pre-LLVM diagnostic breadcrumb and fix Windows CI build configuration

Two changes in one task since they're both small and locally verifiable:

1. **Add pre-LLVM init breadcrumb to build trace:**
   - In `compiler/mesh-codegen/src/lib.rs`, add a `build_trace::set_stage("pre-llvm-init")` call immediately before the first `Context::create()` in `compile_to_binary` and `compile_mir_to_binary`
   - This ensures the next Windows crash (if any) has a recorded phase in the build trace

2. **Fix the release.yml Windows build configuration:**
   - Remove the `RUSTFLAGS: -Clink-args=/FORCE:MULTIPLE` line from the Windows build step (line 179)
   - Remove the vcpkg libxml2 install step (lines 121-128) — the LLVM 21 prebuilt tarball is self-contained and doesn't need an external libxml2. The duplicate symbols come from both LLVM's bundled XML2 and the vcpkg copy being linked
   - Remove the `LIB` env var addition that pointed to the vcpkg lib directory
   - If LLVM's `llvm-config --system-libs` on Windows still wants libxml2, provide it from the LLVM tarball's own lib directory instead of vcpkg

3. **Verify locally:**
   - `cargo test -p mesh-codegen link -- --nocapture` still passes
   - `rg 'FORCE:MULTIPLE' .github/workflows/` returns no matches
   - The workflow YAML is valid (no syntax errors)

## Inputs

- `.github/workflows/release.yml`
- `compiler/mesh-codegen/src/lib.rs`
- `.tmp/m034-s12/t01/diagnostic-summary.json`

## Expected Output

- `Modified .github/workflows/release.yml without /FORCE:MULTIPLE`
- `Modified compiler/mesh-codegen/src/lib.rs with pre-LLVM breadcrumb`

## Verification

cargo test -p mesh-codegen link -- --nocapture && ! rg -q 'FORCE:MULTIPLE' .github/workflows/release.yml
