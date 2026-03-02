---
phase: 147-publish-and-verify
plan: 03
subsystem: infra
tags: [meshpkg, meshc, install, consumer, e2e, discovery, packages]

# Dependency graph
requires:
  - phase: 147-publish-and-verify/01
    provides: fixed meshpkg tarball and initial meshc installed package discovery
  - phase: 147-publish-and-verify/02
    provides: snowdamiz/mesh-slug@1.0.0 live on registry at api.packages.meshlang.dev
provides:
  - tests/e2e/consumer/ project demonstrating full install+use workflow
  - meshpkg install working for scoped package names ({owner}/{name}@{version})
  - meshc build working for consumer projects with scoped installed packages
  - Consumer binary printing "hello-world" end-to-end proof
affects: [148-integrate]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Scoped packages install to .mesh/packages/{owner}/{name}@{version}/ (two-level layout)"
    - "discovery.rs Phase 1b must iterate two levels: packages/ -> owner/ -> name@version/"
    - "meshpkg install (no args) reads mesh.toml dependencies and installs all from default registry"
    - "Default registry is https://api.packages.meshlang.dev (not registry.meshlang.dev)"

key-files:
  created:
    - tests/e2e/consumer/mesh.toml
    - tests/e2e/consumer/main.mpl
    - tests/e2e/consumer/mesh.lock
    - tests/e2e/consumer/.gitignore
  modified:
    - compiler/meshc/src/discovery.rs

key-decisions:
  - "Scoped package install layout is {owner}/{name}@{version}/ — discovery.rs must walk two levels deep"
  - "Default meshpkg registry URL is https://api.packages.meshlang.dev (registry.meshlang.dev is a Vercel frontend that 404s)"
  - "Consumer project .mesh/ and binary excluded from git via per-project .gitignore"

patterns-established:
  - "E2E consumer pattern: mesh.toml with dependency + main.mpl importing module by PascalCase name"
  - "meshpkg install (no pkg name) installs all mesh.toml dependencies from default registry"

requirements-completed: [DIST-04]

# Metrics
duration: 3min
completed: 2026-03-02
---

# Phase 147 Plan 03: E2E Consumer Install and Verification Summary

**End-to-end install+run verified: meshpkg install snowdamiz/mesh-slug@1.0.0, meshc build consumer, binary prints "hello-world" — plus scoped package discovery fix in discovery.rs**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-02T05:21:40Z
- **Completed:** 2026-03-02T05:24:30Z
- **Tasks:** 1 auto (+ checkpoint 2 returned to orchestrator)
- **Files modified:** 5 (4 created, 1 modified)

## Accomplishments

- Created tests/e2e/consumer/ with mesh.toml (snowdamiz/mesh-slug = "1.0.0"), main.mpl (from Slug import slugify), and mesh.lock
- meshpkg install (no args) downloaded and installed snowdamiz/mesh-slug@1.0.0 to .mesh/packages/snowdamiz/mesh-slug@1.0.0/
- Fixed discovery.rs to support scoped package paths (two-level owner/name@version layout)
- meshc build compiled consumer project without errors
- Consumer binary ran and printed "hello-world"

## Task Commits

Each task was committed atomically:

1. **Task 1: Create consumer project, install mesh-slug, compile, and run** - `4f1cc0a2` (feat)

## Files Created/Modified

- `tests/e2e/consumer/mesh.toml` - Consumer project manifest with snowdamiz/mesh-slug = "1.0.0" dependency
- `tests/e2e/consumer/main.mpl` - Consumer program: from Slug import slugify; prints slugify("Hello World!")
- `tests/e2e/consumer/mesh.lock` - Lockfile with snowdamiz/mesh-slug@1.0.0 SHA256 checksum
- `tests/e2e/consumer/.gitignore` - Excludes .mesh/ and compiled binary from git
- `compiler/meshc/src/discovery.rs` - Fix Phase 1b to handle scoped package paths (two-level layout)

## Decisions Made

- Registry URL in `meshpkg --help` default is `https://api.packages.meshlang.dev`; `registry.meshlang.dev` returns Vercel DEPLOYMENT_NOT_FOUND — always use the default (no --registry flag needed)
- Scoped package install path is `.mesh/packages/{owner}/{name}@{version}/` — two levels deep under packages/
- Per-project .gitignore in tests/e2e/consumer/ rather than adding consumer-specific rules to root .gitignore

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed discovery.rs Phase 1b to handle scoped package paths**
- **Found during:** Task 1 (meshc build step)
- **Issue:** Plan 01's discovery fix assumed unscoped layout `.mesh/packages/{name}@{version}/`. After publish with scoped name, meshpkg installs to `.mesh/packages/snowdamiz/mesh-slug@1.0.0/`. Discovery iterated packages/ and found `snowdamiz/` as the package dir — called `discover_mesh_files` on it, found `mesh-slug@1.0.0/slug.mpl`, and `path_to_module_name` produced `MeshSlug10.Slug` instead of `Slug`. Module `Slug` not found.
- **Fix:** Two-level iteration in Phase 1b: if a packages/ entry contains `@` in name, treat it as a versioned pkg dir (unscoped). Otherwise, treat it as an owner dir and iterate its subdirs for entries containing `@` (scoped layout). Both layouts are now supported.
- **Files modified:** `compiler/meshc/src/discovery.rs`
- **Verification:** `meshc build tests/e2e/consumer` exits 0, prints "Compiled: tests/e2e/consumer/consumer"; `tests/e2e/consumer/consumer` prints "hello-world"
- **Committed in:** `4f1cc0a2`

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Essential correctness fix. The Plan 01 discovery implementation was designed for unscoped packages; scoped packages (the actual registry format) require an additional level of directory traversal.

## Issues Encountered

- `registry.meshlang.dev` returns Vercel `DEPLOYMENT_NOT_FOUND` for all API requests — this is the registry frontend URL, not the API backend. The `meshpkg` default registry (`api.packages.meshlang.dev`) works correctly. Plan context referenced the wrong URL; meshpkg's default was used instead.

## User Setup Required

None - no external service configuration required for this plan.

## Next Phase Readiness

- Full Phase 147 success criteria met pending human verification of packages.meshlang.dev visual search
- Checkpoint 2 returned: human verifies 4 criteria (mesh.toml, install dir, compile, runtime output, website search)
- Phase 148 (integrate) can proceed once checkpoint approved

---
*Phase: 147-publish-and-verify*
*Completed: 2026-03-02*
