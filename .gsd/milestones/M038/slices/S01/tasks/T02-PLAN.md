---
estimated_steps: 17
estimated_files: 1
skills_used: []
---

# T02: Push to CI and verify hosted release lane

Push the changes and verify the hosted release workflow:

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

## Inputs

- `T01 output`
- `hosted CI run results`

## Expected Output

- `Green hosted release.yml run`
- `All six workflow lanes passing`

## Verification

Hosted `release.yml` `Verify release assets (x86_64-pc-windows-msvc)` job goes green. All six hosted workflow lanes pass.
