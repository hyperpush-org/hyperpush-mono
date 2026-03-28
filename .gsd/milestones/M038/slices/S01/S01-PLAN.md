# S01: Fix Windows MSVC Build and Verify Release Lane

**Goal:** Remove `/FORCE:MULTIPLE` from the Windows CI build, fix the underlying duplicate symbol issue (likely vcpkg libxml2 conflicting with LLVM static libs), add pre-LLVM diagnostic breadcrumbs, and verify the hosted release lane goes green.
**Demo:** After this: The hosted `release.yml` Windows smoke job goes green. `meshc.exe build` on the trivial fixture produces a working executable without `/FORCE:MULTIPLE`.

## Tasks
- [x] **T01: Added pre-LLVM-init build trace stage and removed vcpkg libxml2 + FORCE:MULTIPLE from Windows CI build** — Two changes in one task since they're both small and locally verifiable:

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
  - Estimate: 30min
  - Files: compiler/mesh-codegen/src/lib.rs, .github/workflows/release.yml
  - Verify: cargo test -p mesh-codegen link -- --nocapture && ! rg -q 'FORCE:MULTIPLE' .github/workflows/release.yml
- [ ] **T02: Push to CI and verify hosted release lane** — Push the changes and verify the hosted release workflow:

1. **Push to trigger CI:**
   - Commit the workflow and codegen changes
   - Push to the branch that triggers `release.yml`
   - If the full-range push times out, use staged fast-forward pushes (per KNOWLEDGE.md pattern)

2. **Monitor the hosted run:**
   - Wait for the `release.yml` run to complete
   - Check the `Verify release assets (x86_64-pc-windows-msvc)` job specifically
   - If it fails: download diagnostic artifacts, read the build trace and linker errors, iterate

3. **If the first attempt fails due to missing libxml2:**
   - The LLVM tarball may still need an external libxml2 for `llvm-config --system-libs`
   - In that case, keep the vcpkg install but avoid adding it to `LIB` globally — instead set `LLVM_SYS_211_PREFIX` to include both the LLVM prefix and the libxml2 path through llvm-sys's expected lookup
   - Or configure llvm-sys with `prefer-dynamic` feature on Windows

4. **Verify all lanes green:**
   - All six hosted workflow lanes must pass
   - Run `scripts/verify-m034-s05.sh` assembly replay if local access to the hosted state is available

This task may require multiple CI round-trips. Each iteration should maximize diagnostic data.
  - Estimate: 2-4h (CI round-trips)
  - Files: .github/workflows/release.yml
  - Verify: Hosted `release.yml` `Verify release assets (x86_64-pc-windows-msvc)` job goes green. All six hosted workflow lanes pass.
