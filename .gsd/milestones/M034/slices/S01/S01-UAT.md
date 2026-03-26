# S01: Real registry publish/install proof — UAT

**Milestone:** M034
**Written:** 2026-03-26T21:42:15.063Z

# S01: Real registry publish/install proof — UAT

**Milestone:** M034

## UAT Type

- UAT mode: mixed
- Why this mode is sufficient: S01 spans both local compiler/editor regressions and a live registry/packages-site replay, so acceptance must combine deterministic local tests with a real publish/install/download/build verification run.

## Preconditions

- Run from the repo root.
- `.env` contains a valid dashboard-issued `MESH_PUBLISH_OWNER` and `MESH_PUBLISH_TOKEN` from `https://packages.meshlang.dev/publish`.
- Network access to `https://api.packages.meshlang.dev` and `https://packages.meshlang.dev` is available.
- Do not run multiple copies of `scripts/verify-m034-s01.sh` at once; the verifier intentionally reuses `.tmp/m034-s01/home` and `.tmp/m034-s01/work`.

## Smoke Test

1. Run `set -a && source .env && set +a && bash scripts/verify-m034-s01.sh`.
2. Confirm the phases print in order: `[contract]`, `[tooling]`, `[auth]`, `[publish]`, `[metadata]`, `[download]`, `[install]`, `[build]`, `[runtime]`, `[duplicate]`, `[visibility]`.
3. **Expected:** the command exits 0 and ends with `verify-m034-s01: ok`.

## Test Cases

### 1. Scoped installed packages build from the natural cache layout

1. Run `cargo test -p meshc --test e2e_m034_s01 scoped_installed_package_builds -- --nocapture`.
2. **Expected:** the test passes and the consumer fixture successfully builds and runs against `.mesh/packages/acme/greeter@1.0.0` without flattening the scoped directory structure.

### 2. Editor analysis matches compiler discovery for scoped installs

1. Run `cargo test -p mesh-lsp scoped_installed_package -- --nocapture`.
2. **Expected:** the scoped and flat installed-package analysis regressions pass, and the discovery test proves that owner-only, hidden, or manifestless directories are skipped instead of being treated as package roots.

### 3. Public docs and CLI contract stay honest about scoped dependencies and named install

1. Run `rg -n '"your-login/your-package" = "1.0.0"' website/docs/docs/tooling/index.md`.
2. Run `rg -n 'does not edit mesh.toml|updates mesh.lock' website/docs/docs/tooling/index.md compiler/meshpkg/src/install.rs`.
3. **Expected:** the tooling docs contain a quoted scoped dependency key example, and both docs plus CLI source say plainly that named install updates `mesh.lock` but does not edit `mesh.toml`.

### 4. Live registry proof succeeds end to end

1. Run `set -a && source .env && set +a && bash scripts/verify-m034-s01.sh`.
2. Let the verifier publish a fresh unique version, fetch package/version/search metadata, download the tarball, install through `mesh.toml`, perform a named install, build and run the consumer binary, POST the duplicate publish, and check the packages site detail/search pages.
3. **Expected:**
   - publish succeeds and the version is visible at the exact registry/package URLs,
   - the downloaded tarball SHA matches publish metadata and `mesh.lock`,
   - the consumer binary prints `registry proof ok`,
   - the duplicate publish is observed as HTTP 409 inside the successful verifier flow,
   - the packages-site detail and search pages contain the package name and version,
   - the verifier exits 0 with `verify-m034-s01: ok`.

### 5. Observability artifacts are preserved for the latest run

1. After a successful verifier run, inspect `.tmp/m034-s01/verify/` and open the newest version directory.
2. Confirm the directory contains phase logs/artifacts such as `00-context.log`, `03-publish.log`, `04-package-meta.log`, `08-download.log`, `09-install.log`, `09b-named-install.log`, `10-consumer-build.log`, `11-consumer-run.log`, `12-duplicate-publish.log`, `13-detail-page-attempt1.log`, and `14-search-page-attempt1.log`.
3. Confirm machine-readable artifacts exist: `publish.json`, `package.json`, `version.json`, `versions.json`, `search.json`, `download.tar.gz`, `download.sha256`, `mesh.lock`, `named-install.json`, `named-install.mesh.lock`, and before/after named-install manifest snapshots.
4. **Expected:** every proof phase leaves enough evidence to diagnose the first failure without re-running blindly.

## Edge Cases

### Missing credentials fail before live publish

1. Run `env -u MESH_PUBLISH_OWNER -u MESH_PUBLISH_TOKEN bash scripts/verify-m034-s01.sh`.
2. **Expected:** the verifier exits non-zero immediately with a clear missing-env message and does not attempt a publish.

### Named install must not mutate `mesh.toml`

1. Run the full live verifier.
2. Compare `named-install.mesh.toml.before` and `named-install.mesh.toml.after` in the latest run directory.
3. **Expected:** the files are byte-for-byte identical while `named-install.mesh.lock` records the fetched package/version/source.

### Duplicate publish is a required success-path check, not a soft warning

1. Open `12-duplicate-publish.status` in the latest run directory after a successful verifier run.
2. **Expected:** the file contains `409`, and the overall verifier still exits 0 because duplicate rejection is part of the asserted contract.

## Failure Signals

- `cargo test -p meshc --test e2e_m034_s01 scoped_installed_package_builds -- --nocapture` fails or starts requiring a flattened package cache layout.
- `cargo test -p mesh-lsp scoped_installed_package -- --nocapture` reports diagnostics for the scoped/flat fixtures or starts treating owner directories as package roots.
- The docs grep loses the quoted scoped dependency example or the named-install `mesh.lock` / no-`mesh.toml` contract text.
- `bash scripts/verify-m034-s01.sh` stops before `verify-m034-s01: ok` or no longer leaves per-phase artifacts under `.tmp/m034-s01/verify/<version>/`.
- The duplicate-publish phase stops returning 409, the packages site no longer shows the new version, or the downloaded tarball SHA diverges from publish metadata / `mesh.lock`.

## Requirements Proved By This UAT

- R007 — Mesh now has a believable reproducible package workflow: real publish, real metadata/search/detail truth, real tarball download, real install, real `mesh.lock` recording, real consumer build/run, and real duplicate-publish rejection on the public registry path.

## Not Proven By This UAT

- CI wiring for the verifier (`S02`).
- Installer/release-asset truth for released binaries (`S03`).
- The full assembled public release lane across binaries, docs deployment, packages site, and extension publication (`S05`).

## Notes for Tester

Use the live verifier as the canonical replay surface. If a future change touches registry API behavior, package-manager output, or the public tooling docs, update the verifier, fixtures, and contract greps in the same task so the proof surface stays singular and honest.
