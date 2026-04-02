# S01: Configurable entrypoint in compiler and test discovery

**Goal:** Replace the compiler/discovery/test-runner hardcoded root-`main.mpl` assumption with one default-plus-override executable entrypoint contract.
**Demo:** After this: After this: a real Mesh project can keep root `main.mpl` or use an overridden entry file such as `lib/start.mpl`, and compiler build plus test discovery both honor the same executable contract.

## Tasks
- [x] **T01: Added manifest-driven entrypoint resolution and entry-aware discovery so non-root executables keep path-derived module names.** — Add the optional `[package].entrypoint` manifest field and one small resolver seam, then make `meshc build` and discovery use the resolved entry path without turning non-root entry modules into a special-case `Main` name.
  - Estimate: 90m
  - Files: compiler/mesh-pkg/src/manifest.rs, compiler/meshc/src/main.rs, compiler/meshc/src/discovery.rs
  - Verify: cargo test -p mesh-pkg entrypoint -- --nocapture && cargo test -p meshc build_project_ -- --nocapture
- [x] **T02: Made `meshc test` honor resolved entrypoints across project-dir, tests-dir, and specific-file targets, and removed the stale compile-time fixture dependency blocking the package verification rail.** — Fix `meshc test` so project-dir, tests-dir, and specific-file targets all resolve the same project root and configured entrypoint, then replace that resolved executable with the synthetic test `main.mpl` instead of assuming root `main.mpl` is the only executable file.
  - Estimate: 90m
  - Files: compiler/meshc/src/test_runner.rs, compiler/meshc/tests/tooling_e2e.rs, compiler/mesh-pkg/src/manifest.rs
  - Verify: cargo test -p meshc --test tooling_e2e test_test_ -- --nocapture
- [x] **T03: Added the M048/S01 acceptance rail and fixed MIR merge so manifest-selected entrypoints win end-to-end.** — Create one named acceptance target that proves build and test discovery stay aligned for default projects and manifest-override projects, including the no-root-main cases that currently fail or false-green.
  - Estimate: 75m
  - Files: compiler/meshc/tests/e2e_m048_s01.rs
  - Verify: cargo test -p meshc --test e2e_m048_s01 -- --nocapture
