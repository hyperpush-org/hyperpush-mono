---
phase: 139-package-manifest-meshpkg-cli
plan: "02"
subsystem: meshpkg
tags: [package-manager, cli, clap, ureq, tar, sha256, credentials]
dependency_graph:
  requires: [PKG-01, PKG-02]
  provides: [PKG-03, PKG-04, PKG-05, PKG-06]
  affects: [compiler/meshpkg, Cargo.toml]
tech_stack:
  added: [tar 0.4, flate2 1, indicatif 0.17, colored 2, dirs 5]
  patterns: [ureq 3 Body.as_reader().read_to_end() for binary, status().as_u16() for match, clap derive with global --json flag, TOML credentials file at ~/.mesh/credentials]
key_files:
  created:
    - compiler/meshpkg/Cargo.toml
    - compiler/meshpkg/src/main.rs
    - compiler/meshpkg/src/auth.rs
    - compiler/meshpkg/src/publish.rs
    - compiler/meshpkg/src/install.rs
    - compiler/meshpkg/src/search.rs
  modified:
    - Cargo.toml
    - compiler/mesh-pkg/Cargo.toml
decisions:
  - "ureq 3 Body.read_to_end() does not implement std::io::Read directly — use body_mut().as_reader().read_to_end() for binary downloads"
  - "response.status() in ureq 3 returns StatusCode not u16 — match via status().as_u16()"
  - "Tasks 1 and 2 committed together because main.rs references all submodules at compile time — same situation as Plan 01"
  - "ureq and sha2 promoted from mesh-pkg direct deps to workspace deps so meshpkg can use them via workspace = true"
metrics:
  duration: "3m 58s"
  completed: "2026-03-01"
  tasks_completed: 2
  files_modified: 8
---

# Phase 139 Plan 02: meshpkg CLI Binary Summary

meshpkg standalone CLI binary with four subcommands (login/publish/install/search) wrapping the mesh-pkg library, using clap derive, ureq 3, sha256 verification, tar/gz packaging, indicatif spinners, and colored output.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Workspace setup and meshpkg crate scaffolding with auth + main | 4c14e80a | Cargo.toml, compiler/mesh-pkg/Cargo.toml, compiler/meshpkg/Cargo.toml, src/main.rs, src/auth.rs |
| 2 | Implement publish, install, and search subcommands | 4c14e80a | compiler/meshpkg/src/publish.rs, src/install.rs, src/search.rs |

Note: Tasks 1 and 2 were committed together. main.rs references all four submodules (mod auth; mod install; mod publish; mod search;), so the crate cannot compile without all of them present. This is the same forced-together situation as Plan 01.

## What Was Built

### Workspace (Cargo.toml)
- Added `compiler/meshpkg` to workspace members
- Promoted `ureq = { version = "3", features = ["gzip"] }` and `sha2 = "0.10"` to workspace-level deps (previously direct deps in mesh-pkg)
- Added new workspace deps: `tar = "0.4"`, `flate2 = "1"`, `indicatif = "0.17"`, `colored = "2"`, `dirs = "5"`
- Updated compiler/mesh-pkg/Cargo.toml to use `{ workspace = true }` for ureq and sha2

### compiler/meshpkg/Cargo.toml
- Binary crate with `[[bin]] name = "meshpkg"`
- All deps via `{ workspace = true }` pattern

### main.rs
- clap derive CLI with global `--json` flag suppressing spinners and colored output
- Four subcommands: Login (--token optional, stdin fallback), Publish (--registry), Install ([NAME] optional, --registry), Search (query, --registry)
- Default registry: `https://registry.meshlang.dev`
- Error printing via `eprintln!("✗ {}", e)` or JSON `{"error":"..."}` depending on --json flag
- `process::exit(0/1)` from main; submodules return `Result<(), String>`

### auth.rs
- `credentials_path()` returns `~/.mesh/credentials`
- `read_token()` reads TOML `[registry] token = "..."` — descriptive error if not logged in
- `write_token(token)` creates `~/.mesh/` dir and writes TOML credentials file

### publish.rs
- `create_tarball()` builds .tar.gz in memory: mesh.toml at archive root + src/ dir
- Computes SHA-256 of the tarball bytes
- `upload_tarball()` POSTs to `/api/v1/packages` with Authorization: Bearer, X-Package-Name, X-Package-Version, X-Package-SHA256 headers
- `with_spinner()` helper: indicatif spinner for human mode, passthrough for --json mode
- HTTP 201/200 = success, 409 = immutable version error, 401 = unauthorized, other = status code error

### install.rs
- `install_all()`: reads mesh.toml dependencies, honors existing mesh.lock pins, downloads + SHA-256 verifies + extracts each registry dep to `.mesh/packages/<name>@<version>/`, writes mesh.lock
- `install_named()`: resolves latest version from registry, downloads + verifies + extracts, updates mesh.lock
- `download_tarball()`: ureq 3 GET, binary read via `body_mut().as_reader().read_to_end()`, SHA-256 digest
- `resolve_version()`: queries `/api/v1/packages/<name>/<version>` for sha256
- `resolve_latest()`: queries `/api/v1/packages/<name>` for latest version + sha256
- `extract_tarball()`: flate2 GzDecoder + tar Archive::unpack()

### search.rs
- `fetch_results()`: ureq 3 GET `/api/v1/packages?search=<query>`, deserializes JSON `[{name, version, description}]`
- `print_search_table()`: right-pads NAME and VERSION columns to max width, prints aligned table
- JSON mode: serializes results array directly

## Verification

```
cargo build -p meshpkg
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
```

All subcommand --help commands show correct usage with flags.

```
cargo run -p meshpkg -- login --token test-token-abc123
✓ Token saved to /Users/sn0w/.mesh/credentials

cat ~/.mesh/credentials
[registry]
token = "test-token-abc123"
```

```
cargo test -p mesh-pkg | tail -3
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Publish with unreachable registry fails with clear error (not a panic):
```
✗ Failed to connect to registry: io: Connection refused
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ureq 3 Body does not implement std::io::Read for read_to_end**
- **Found during:** Task 1 verification
- **Issue:** `response.body_mut().read_to_end(&mut buf)` fails because `Body` does not implement `std::io::Read` directly in ureq 3
- **Fix:** Changed to `response.body_mut().as_reader().read_to_end(&mut buf)` which uses `BodyReader` that does implement `std::io::Read`; added `use std::io::Read as IoRead` import
- **Files modified:** compiler/meshpkg/src/install.rs
- **Commit:** 4c14e80a (fixed inline before commit)

**2. [Rule 1 - Bug] ureq 3 status() returns StatusCode not u16**
- **Found during:** Task 1 verification
- **Issue:** `match response.status() { 200 | 201 => ...` fails — `response.status()` returns `ureq::http::StatusCode`, not an integer
- **Fix:** Changed to `match response.status().as_u16() { 200 | 201 => ...` consistent with existing mesh-rt/src/http/client.rs pattern
- **Files modified:** compiler/meshpkg/src/publish.rs
- **Commit:** 4c14e80a (fixed inline before commit)

**3. [Structural] Tasks 1 and 2 committed together**
- **Issue:** main.rs declares `mod auth; mod install; mod publish; mod search;` — all four submodule files must exist for the crate to compile; cannot commit Task 1 without Task 2's files already present
- **Fix:** Implemented all files before first commit; all changes in single commit 4c14e80a
- **Impact:** Functionally identical to two separate commits

## Self-Check: PASSED

Files exist:
- compiler/meshpkg/Cargo.toml — FOUND
- compiler/meshpkg/src/main.rs — FOUND
- compiler/meshpkg/src/auth.rs — FOUND
- compiler/meshpkg/src/publish.rs — FOUND
- compiler/meshpkg/src/install.rs — FOUND
- compiler/meshpkg/src/search.rs — FOUND

Commit 4c14e80a — FOUND (cargo build succeeds, all tests pass)
