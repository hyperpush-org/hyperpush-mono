---
id: S01
parent: M034
milestone: M034
provides:
  - A passing authoritative live proof command: `bash scripts/verify-m034-s01.sh`, which now proves real registry publish/install/download/lockfile/build/visibility truth end to end.
  - A stable scoped installed-package discovery contract for `.mesh/packages/<owner>/<package>@<version>` shared by `meshc` and `mesh-lsp`.
  - An honest named-install contract for `meshpkg install <name>`: fetch and lock the latest release, update `mesh.lock`, keep `mesh.toml` unchanged, and surface that behavior through JSON, CLI text, docs, and verifier assertions.
  - Blob-first registry publish ordering and download-before-counter semantics that prevent metadata or download truth from getting ahead of stored package bytes.
requires:
  []
affects:
  - S02
  - S03
  - S05
key_files:
  - compiler/meshc/src/discovery.rs
  - compiler/mesh-lsp/src/analysis.rs
  - compiler/meshc/tests/e2e_m034_s01.rs
  - scripts/verify-m034-s01.sh
  - scripts/fixtures/m034-s01-proof-package/mesh.toml.template
  - scripts/fixtures/m034-s01-proof-package/registry_proof.mpl
  - scripts/fixtures/m034-s01-consumer/mesh.toml.template
  - scripts/fixtures/m034-s01-consumer/main.mpl
  - compiler/meshpkg/src/install.rs
  - registry/src/routes/publish.rs
  - registry/src/routes/download.rs
  - registry/src/db/packages.rs
  - website/docs/docs/tooling/index.md
  - .gsd/DECISIONS.md
  - .gsd/PROJECT.md
  - .gsd/REQUIREMENTS.md
key_decisions:
  - Use one canonical repo-local verifier (`scripts/verify-m034-s01.sh`) as the authoritative live proof surface for publish → metadata/search/detail → download checksum → install → `mesh.lock` truth → consumer build/run → duplicate publish rejection → packages-site visibility.
  - Keep the natural scoped cache layout under `.mesh/packages/<owner>/<package>@<version>` and align `meshc` plus `mesh-lsp` on the same manifest-leaf package-root discovery rule instead of flattening scoped names.
  - Establish package blob truth before inserting the registry version row so public metadata cannot go green ahead of stored package bytes.
  - Treat `meshpkg install <name>` as a fetch-plus-lock operation that updates `mesh.lock` but does not edit `mesh.toml`, and make that contract mechanically visible in CLI JSON, docs, and verifier checks.
patterns_established:
  - For registry/release proof work, keep one canonical verifier script with checked-in fixtures, deterministic temp roots, unique per-run versions, and phase-specific logs instead of scattering the proof across ad-hoc shell commands or CI YAML.
  - Keep compiler and editor package discovery aligned on the same manifest-leaf rule so scoped-install regressions fail close to root cause and not later as misleading module-resolution noise.
  - Make user-facing CLI contracts mechanically provable: expose machine-readable JSON fields, preserve before/after manifests when mutation is forbidden, and pair public docs with explicit grep checks instead of trusting prose.
  - Use the same verified HTTPS transport stack across all live verifier phases; mixing transports can create host-local TLS false negatives that hide real product truth.
observability_surfaces:
  - Per-phase verifier logs and HTTP artifacts under `.tmp/m034-s01/verify/<version>/`, including `00-context.log`, `03-publish.log`, `04-package-meta.*`, `05-version-meta.*`, `06-versions.*`, `07-search.*`, `08-download.*`, `09-install.log`, `09b-named-install.log`, `10-consumer-build.log`, `11-consumer-run.log`, `12-duplicate-publish.*`, `13-detail-page-attempt*.log`, and `14-search-page-attempt*.log`.
  - Verifier output artifacts that make state auditable after the run: `publish.json`, `package.json`, `version.json`, `versions.json`, `search.json`, `download.tar.gz`, `download.sha256`, `install.json`, `named-install.json`, `mesh.lock`, `named-install.mesh.lock`, and before/after manifest snapshots for named install.
  - Compiler/LSP regressions that surface scoped-install discovery drift early: `cargo test -p meshc --test e2e_m034_s01 scoped_installed_package_builds -- --nocapture` and `cargo test -p mesh-lsp scoped_installed_package -- --nocapture`.
  - Public contract greps that fail docs/CLI drift mechanically: `rg -n '"your-login/your-package" = "1.0.0"' website/docs/docs/tooling/index.md` and `rg -n 'does not edit mesh.toml|updates mesh.lock' website/docs/docs/tooling/index.md compiler/meshpkg/src/install.rs`.
drill_down_paths:
  - .gsd/milestones/M034/slices/S01/tasks/T01-SUMMARY.md
  - .gsd/milestones/M034/slices/S01/tasks/T02-SUMMARY.md
  - .gsd/milestones/M034/slices/S01/tasks/T03-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-03-26T21:42:15.062Z
blocker_discovered: false
---

# S01: Real registry publish/install proof

**Closed the real Mesh registry proof gap by making scoped published packages build from their natural cache layout and by landing a live verifier that rechecks publish, metadata, download, install, lockfile, named-install, duplicate-publish, and public-visibility truth against the real registry path.**

## What Happened

S01 turned the Mesh package path from an assumed release story into a real, repeatable proof. T01 closed the underlying compiler/editor blocker by teaching both `meshc` and `mesh-lsp` to walk `.mesh/packages` until they reach the first descendant directory containing `mesh.toml`, so naturally nested scoped installs such as `.mesh/packages/acme/greeter@1.0.0` are treated as package roots instead of owner directories. That change preserved the real on-disk cache layout, kept package-root `main.mpl` out of normal module resolution, and pinned the behavior with compiler and LSP regressions that fail close to discovery drift instead of later as vague import errors.

T02 then built the canonical proof surface for the slice: checked-in proof-package and consumer fixtures plus `scripts/verify-m034-s01.sh`. The verifier isolates credentials and workspaces under `.tmp/m034-s01/`, renders deterministic manifests with quoted scoped dependency keys, builds local `meshpkg`/`meshc` once, publishes a unique version through the live registry with `meshpkg --json`, rechecks metadata/version/search responses directly over HTTP, downloads the tarball and verifies its SHA-256, installs from `mesh.toml`, checks `mesh.lock`, performs a named install that must leave `mesh.toml` untouched, builds and runs a consumer binary, confirms duplicate publish rejection, and checks packages-site detail/search visibility from the exact package pages instead of trusting homepage counts.

T03 hardened the remaining truth edges so the verifier could not go green on half-committed or misleading behavior. Registry publish now establishes blob truth in storage before a version row becomes public metadata truth. Downloads now fetch the blob before incrementing counters, and the DB helper that updates package/version counters does so transactionally. `meshpkg install <name>` is now explicitly an install-plus-lock operation: it reports `lockfile` and `manifest_changed: false`, prints honest follow-up guidance for declaring dependencies, and the public tooling docs now show quoted scoped dependency keys plus the no-hidden-`mesh.toml`-edit contract.

Closeout verified the assembled slice with the real registry path and removed the final verifier-local blocker: the duplicate-publish POST now uses `curl`, matching the trusted transport used by the successful GET phases. After that repair, `set -a && source .env && set +a && bash scripts/verify-m034-s01.sh` completed successfully for `snowdamiz/mesh-registry-proof@0.34.0-20260326213540-36652`, including the live duplicate-publish 409 check and packages-site visibility checks. The latest successful run directory is `.tmp/m034-s01/verify/0.34.0-20260326213540-36652/`.

## Operational Readiness
- **Health signal:** `bash scripts/verify-m034-s01.sh` exits 0 and prints `verify-m034-s01: ok`; the run directory contains `publish.json`, `package.json`, `version.json`, `search.json`, `download.tar.gz`, `download.sha256`, `mesh.lock`, `named-install.json`, and phase logs through `14-search-page-attempt1.log`.
- **Failure signal:** the verifier stops on the first failing phase, prints `verification drift`, `first failing phase`, and the package/version coordinate, and leaves the failing phase’s log/body/header artifacts under `.tmp/m034-s01/verify/<version>/`.
- **Recovery procedure:** inspect the failing phase log in the run directory, correct the specific contract drift, and rerun the verifier with valid owner/token credentials. The script generates a fresh version automatically, so reruns do not need manual version cleanup. Keep the duplicate-publish phase on the same curl-backed transport as the rest of the live HTTPS checks.
- **Monitoring gaps:** there is not yet an always-on CI lane or scheduled drift monitor that reruns this live proof automatically. Packages-site visibility and registry-path truth are currently checked on demand via the verifier, and later M034 slices must promote this into continuous release verification.

## Verification

All slice-plan verification checks now pass. Local compiler/editor proof: `cargo test -p meshc --test e2e_m034_s01 scoped_installed_package_builds -- --nocapture` passed, and `cargo test -p mesh-lsp scoped_installed_package -- --nocapture` passed. Contract surfaces: `bash -n scripts/verify-m034-s01.sh` passed, `rg -n '"your-login/your-package" = "1.0.0"' website/docs/docs/tooling/index.md` passed, and `rg -n 'does not edit mesh.toml|updates mesh.lock' website/docs/docs/tooling/index.md compiler/meshpkg/src/install.rs` passed. Live registry proof: `set -a && source .env && set +a && bash scripts/verify-m034-s01.sh` passed end to end and produced a successful run directory at `.tmp/m034-s01/verify/0.34.0-20260326213540-36652/`, confirming publish metadata, search/detail visibility, tarball checksum, install, named-install manifest stability, `mesh.lock` truth, consumer build/run, duplicate publish 409, and retained observability artifacts.

## Requirements Advanced

None.

## Requirements Validated

- R007 — `cargo test -p meshc --test e2e_m034_s01 scoped_installed_package_builds -- --nocapture`, `cargo test -p mesh-lsp scoped_installed_package -- --nocapture`, `bash -n scripts/verify-m034-s01.sh`, `rg -n '"your-login/your-package" = "1.0.0"' website/docs/docs/tooling/index.md`, `rg -n 'does not edit mesh.toml|updates mesh.lock' website/docs/docs/tooling/index.md compiler/meshpkg/src/install.rs`, and `set -a && source .env && set +a && bash scripts/verify-m034-s01.sh` all passed, and the live verifier completed successfully for `snowdamiz/mesh-registry-proof@0.34.0-20260326213540-36652`.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

Closeout required one verifier-local repair beyond the task summaries: `scripts/verify-m034-s01.sh` now uses `curl` instead of Python `urllib.request` for the duplicate-publish POST so the duplicate check follows the same trusted TLS path as the successful metadata/download phases. No slice replan was needed.

## Known Limitations

The proof still depends on a real dashboard-issued `MESH_PUBLISH_OWNER` / `MESH_PUBLISH_TOKEN` in `.env` for reruns; there is no anonymous live-registry replay. The verifier is intentionally serial and should not be run concurrently because it reuses `.tmp/m034-s01/home` and `.tmp/m034-s01/work`. CI wiring, installer/release-asset truth, and broader public-release assembly are still downstream M034 work in S02/S03/S05.

## Follow-ups

Wire `bash scripts/verify-m034-s01.sh` into the authoritative CI/release lane in S02 instead of re-implementing a second proof path. Reuse the same verifier output and package-manager contract in S03/S05 when validating release assets and full public release assembly. If the packages site, registry API, or CLI contract changes later, update the verifier, fixtures, and docs together so this slice remains the single source of truth.

## Files Created/Modified

- `compiler/meshc/src/discovery.rs` — Added manifest-leaf installed-package discovery so scoped packages under `.mesh/packages/<owner>/<package>@<version>` resolve from the real package root instead of the owner directory.
- `compiler/mesh-lsp/src/analysis.rs` — Mirrored the same manifest-leaf package discovery semantics in editor analysis and added scoped/flat installed-package regressions.
- `compiler/meshc/tests/e2e_m034_s01.rs` — Added the end-to-end compiler regression that builds and runs a consumer against a naturally nested scoped installed package.
- `compiler/mesh-lsp/Cargo.toml` — Added `tempfile` dev support for the new temp-project LSP regressions.
- `scripts/fixtures/m034-s01-proof-package/mesh.toml.template` — Added deterministic proof-package and consumer fixtures that the live verifier renders into isolated workspaces.
- `scripts/fixtures/m034-s01-proof-package/registry_proof.mpl` — Added the proof module exported by the live scoped package fixture.
- `scripts/fixtures/m034-s01-consumer/mesh.toml.template` — Added the quoted scoped dependency fixture for the consumer proof workspace.
- `scripts/fixtures/m034-s01-consumer/main.mpl` — Added the consumer main module used to prove install-plus-build truth against the published package.
- `scripts/verify-m034-s01.sh` — Added the authoritative live verifier and then repaired its duplicate-publish phase to use curl-backed HTTPS so the full publish→metadata→download→install→lockfile→build→duplicate→visibility proof now passes end to end.
- `compiler/meshpkg/src/install.rs` — Made named install explicitly a fetch-plus-lock operation, added machine-checkable JSON fields (`lockfile`, `manifest_changed: false`), and surfaced honest follow-up guidance for declared dependencies.
- `registry/src/routes/publish.rs` — Reordered publish so blob truth is established in object storage before the version row becomes public registry truth.
- `registry/src/routes/download.rs` — Reordered downloads so object fetch succeeds before counters advance, keeping missing-blob failures visible instead of inflating download truth.
- `registry/src/db/packages.rs` — Fixed the package download-counter helper so the version/package counter updates happen transactionally.
- `website/docs/docs/tooling/index.md` — Updated the public tooling docs to use quoted scoped dependency keys and to say plainly that named install updates `mesh.lock` but does not edit `mesh.toml`.
- `.gsd/REQUIREMENTS.md` — Validated R007 from the slice evidence instead of leaving package-workflow trust provisional.
- `.gsd/DECISIONS.md` — Recorded the slice’s missing contract decisions: blob-first publish ordering, manifest-stable named install, and the resulting requirement validation.
- `.gsd/PROJECT.md` — Refreshed project state so M034 now reflects the real-registry proof as complete groundwork for later CI/release slices.
