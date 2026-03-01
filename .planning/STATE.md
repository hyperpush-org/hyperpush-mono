---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: Ecosystem & Standard Library
status: unknown
last_updated: "2026-03-01T23:06:54.942Z"
progress:
  total_phases: 133
  completed_phases: 132
  total_plans: 351
  completed_plans: 348
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-28)

**Core value:** Expressive, readable concurrency -- writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.
**Current focus:** Phase 145 — Redesigning packages-website to production-grade with Tailwind v4 OKLCH design system

## Current Position

Phase: 145 of 145 (Completely redesign the mesh packages page) — 1 of 3 plans COMPLETE
Plan: 1 of 3 — DONE
Status: Phase 145 Plan 01 complete — Tailwind v4 + OKLCH design system installed into packages-website; search API bug fixed (?search= param); registry versions endpoint added at GET /api/v1/packages/{name}/versions
Last activity: 2026-03-01 — Phase 145 Plan 01 complete: Tailwind v4 foundation and API bug fixes in place; build passes; cargo check passes

Progress: [██████████] 100%  (1/3 plans for phase 145)

## Performance Metrics

**All-time Totals (through v13.0):**
- Plans completed: 362
- Phases completed: 134
- Milestones shipped: 23 (v1.0-v13.0)

**v14.0 plan (13 plans across 6 phases):**

| Phase | Plans | Status |
|-------|-------|--------|
| 135. Encoding & Crypto Stdlib | 2 | Complete |
| 136. DateTime Stdlib | 2 | Complete |
| 137. HTTP Client Improvements | 2 | Complete |
| 138. Testing Framework | 5 | Complete (incl. gap closure) |
| 139. Package Manifest & meshpkg CLI | 2 | Complete |
| 140. Package Registry Backend & Website | 4 | In Progress (1/4 summaries) |
| Phase 140 P01 | 12 | 2 tasks | 19 files |
| Phase 140 P02 | 3 | 2 tasks | 7 files |
| Phase 141 P01 | 1 | 2 tasks | 2 files |
| Phase 141 P02 | 5 | 2 tasks | 3 files |
| Phase 141 P03 | 3 | 2 tasks | 1 file |
| Phase 142 P01 | 109 | 3 tasks | 3 files |
| Phase 142 P02 | 2 | 3 tasks | 3 files |
| Phase 142 P03 | 2 | 2 tasks | 2 files |
| Phase 143 P02 | 2 | 2 tasks | 12 files |
| Phase 143 P03 | 2 | 2 tasks | 2 files |
| Phase 143-deploy-everything-including-new-stuff-from-v14 P01 | 1 | 2 tasks | 5 files |
| Phase 143 P03 | 2 | 2 tasks | 2 files |
| Phase 143 P04 | 25 | 3 tasks | 6 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v14.0 Research]: DateTime uses i64 Unix milliseconds — not an opaque heap handle, not strings; avoids new type machinery in typeck/codegen
- [v14.0 Research]: HTTP streaming uses dedicated OS thread per stream (WS reader pattern from v4.0), NOT blocking inside actor coroutines — prevents scheduler deadlock
- [v14.0 Research]: Each *.test.mpl is a complete Mesh program; runner compiles and executes each independently — no function-level test injection
- [v14.0 Research]: Registry package versions are immutable from day one; HTTP 409 on duplicate publish
- [v14.0 Research]: Exact versions only in mesh.toml (no SemVer range solving in v14.0)
- [v14.0 Research]: Coverage (TEST-10) treated as stretch/stub in Phase 138 — MIR counter injection approach; defer full impl to v14.1
- [v14.0 Roadmap]: Phase 135 (Crypto+Encoding) and Phase 136 (DateTime) and Phase 137 (HTTP) and Phase 138 (Testing) all depend only on Phase 134 — can be developed in any order
- [v14.0 Roadmap]: Phase 139 (PKG) depends on Phase 138 (testing framework useful before publishing) — but also logically follows registry API contract
- [v14.0 Roadmap]: Phase 140 (Registry) depends on Phase 139 — manifest format must be finalized before API contract
- [Phase 135 Plan 01]: stdlib module requires 5 registration points: builtins.rs, infer.rs stdlib_modules() HashMap, infer.rs STDLIB_MODULE_NAMES, lower.rs STDLIB_MODULES + map_builtin_name + known_functions, intrinsics.rs LLVM declarations
- [Phase 135 Plan 01]: HMAC-SHA256 RFC 2202 test vector for ("Jefe", "what do ya want for nothing?") = 5bdcc146...ec3843 (not a72840 as in plan)
- [Phase 135 Plan 02]: Base64.decode lenient order: try STANDARD (padded) first, then STANDARD_NO_PAD — if unpadded first, padded input gets bytes incorrectly stripped
- [Phase 135 Plan 02]: Hex.decode is case-insensitive via to_lowercase() before parsing — accepts both lowercase and uppercase hex digits
- [Phase 136 Plan 01]: DateTime ABI is i64 Unix milliseconds throughout — avoids new type machinery in typeck/codegen
- [Phase 136 Plan 01]: diff() return type is MirType::Float (f64) not MirType::Int — fractional precision for sub-second computations
- [Phase 136 Plan 01]: before?/after? retain ? in Mesh source names but drop ? in C symbol names (C cannot contain ?)
- [Phase 136 Plan 01]: alloc_result Ok i64 payload boxed via Box::into_raw(Box::new(ms)) as *mut u8 — same pattern as SqliteConn
- [Phase 136 Plan 02]: Opaque named types backed by scalar ABI must be registered as MirType::Int in resolve_con (not MirType::Struct)
- [Phase 136 Plan 02]: should_deref_boxed_payload must cover MirType::Int/Float/Bool — any scalar type returned via Box::into_raw needs deref in Ok pattern binding
- [Phase 136 Plan 02]: Atom literals lower to bare names without colon — atom_text() strips leading ':'; runtime match arms use "day" not ":day"
- [Phase 136 Plan 02]: is_before/is_after preferred over before?/after? — '?' is Mesh try-operator and 'after' is AFTER_KW (receive-timeout); both block parsing
- [Phase 137]: Atom type for Http.build method param — :get/:post literals type-check correctly; same pattern as DateTime.add :day/:hour
- [Phase 137]: MeshRequest handle is u64 ABI (MirType::Int) via Box::into_raw — same as SqliteConn pattern
- [Phase 137]: http_status_as_error(false) set at Agent level via Agent::config_builder() — ureq 3 removed per-request setting
- [Phase 137 Plan 02]: OS-thread-per-stream mandatory — std::thread::spawn for Http.stream to avoid blocking M:N scheduler; same WS reader pattern from v4.0
- [Phase 137 Plan 02]: Peek-without-drop for cancel — mesh_http_cancel reads Arc via raw reference (not Box::from_raw) to avoid dropping Arc while stream thread holds its clone
- [Phase 137 Plan 02]: usize bridge for *mut u8 across thread boundary — cast to usize before spawn, cast back inside closure; same pattern as ws/server.rs
- [Phase 137 Plan 02]: :continue is a Mesh reserved keyword (loop control) — fixture closures return "ok" string instead; only "stop" is checked by is_stop_atom
- [Phase 137 Plan 02]: Multi-statement closure bodies need 'fn param do ... end' syntax, not arrow form 'fn param -> ...'
- [Phase 138 Plan 01]: test_runner copies each *.test.mpl to temp dir as main.mpl to reuse existing build() entry-point lookup without modification
- [Phase 138 Plan 01]: TestSummary.passed is public API (future plans will read it); suppress dead_code lint with #[allow(dead_code)] on struct
- [Phase 138]: Test module registered with empty HashMap in stdlib_modules() — mock_actor signature deferred to Plan 03
- [Phase 138]: assert_* helpers call fail_with() then panic!() to unwind — Plan 03 harness wraps each test body in catch_unwind
- [Phase 138]: FAIL_MESSAGES thread_local accumulates failure entries; mesh_test_summary reprints them in Failures: section before count line
- [Phase 138 Plan 04]: emit_non_test_items uses tokenize_test_source() for token-based depth tracking — skipping=true + skip_depth=0 waits for opening Do, then tracks nested blocks until skip_depth returns to 0
- [Phase 138 Plan 04]: extract_tests_from_describe helper added — walks describe body with explicit setup/teardown skip logic; avoids premature End detection that would have silently dropped tests
- [Phase 138 Plan 05]: assert_receive preprocessor generates single-line receive blocks — parse_receive_expr does NOT call eat_newlines() before END_KW after after clause; multi-line form fails
- [Phase 138 Plan 05]: ACTOR_MSG_TYPE_KEY injected for __test_body_ functions in infer_fn_def — test body fns are plain fns but main thread has actor PID via mesh_rt_init_actor; receive/self() valid at runtime
- [Phase 138 Plan 05]: test_fail_msg must be registered in builtins.rs as String->Unit — generated assert_receive after-clause calls it; missing registration caused "undefined variable: test_fail_msg"
- [Phase 138 Plan 05]: self() (function call) not bare self for actor PID in test bodies — bare self is NAME_REF (impl receiver); self() is SelfExpr (actor PID using ACTOR_MSG_TYPE_KEY)
- [Phase 138 Plan 05]: Default assert_receive timeout is 100ms when no second argument provided
- [Phase 139 Plan 01]: RegistryShorthand must be FIRST in Dependency enum for serde untagged — bare string "1.0.0" must match before Git/Path are tried
- [Phase 139 Plan 01]: LockedPackage.version is String (not Option<String>) with #[serde(default)] so old lockfiles deserialize to empty string
- [Phase 139 Plan 01]: Registry deps in resolve_deps() return error directing to meshpkg install — network resolution belongs in CLI binary (Plan 02)
- [Phase 139 Plan 02]: ureq 3 Body does not implement std::io::Read — use body_mut().as_reader().read_to_end() for binary downloads (tarball bytes)
- [Phase 139 Plan 02]: response.status() in ureq 3 returns StatusCode not u16 — match via status().as_u16() (consistent with mesh-rt/http/client.rs)
- [Phase 139 Plan 02]: meshpkg credentials stored as TOML at ~/.mesh/credentials with [registry] section; dirs crate for home_dir()
- [Phase 140]: Query param ?name= for per-package pages — VitePress SSG cannot enumerate registry packages at build time; runtime URLSearchParams is the correct pattern
- [Phase 140]: ClientOnly wrapping mandatory for packages components — components access window.location.search and fetch(), unavailable during VitePress SSG build
- [Phase 140]: registry/ excluded from main Cargo workspace to avoid libsqlite3-sys links conflict with mesh-rt's bundled sqlite3; has own [workspace] root
- [Phase 140]: Runtime sqlx::query() used over compile-time query\!() macros — no live DB available for cargo sqlx prepare during scaffold; upgrade deferred to runtime
- [Phase 140 Plan 03]: PostgresStore from tower-sessions-sqlx-store crate (not tower-sessions itself) — import path is tower_sessions_sqlx_store::PostgresStore
- [Phase 140 Plan 03]: time::Duration::days(30) for session expiry — tower-sessions uses the time crate (0.3), not std::time; explicit time dep required
- [Phase 140 Plan 03]: reqwest 0.11 added as explicit dep for GitHub user API calls — matches transitive version pulled in by oauth2 4.4
- [Phase 140 Plan 03]: session.remove::<String>(SESSION_CSRF) called immediately after retrieval — prevents CSRF replay attacks
- [Phase 140]: Runtime sqlx::query_as() for user UUID lookup in publish (no sqlx::query! macro — no live DB or .sqlx/ cache)
- [Phase 140]: extract_readme_from_tarball case-insensitive match (readme.md) for README.md/readme.md/Readme.md variants using flate2+tar
- [Phase 140]: DefaultBodyLimit::max(50MB) applied via .layer() on publish route chain (not globally)
- [Phase 141]: Crypto.uuid4() for Mesher tokens: API key format changes from 53 to 41 chars (mshr_+UUID4); session uses two stripped UUIDs for 64-char hex; bcrypt preserved via pgcrypto — no stdlib equivalent
- [Phase 141]: validate_level exposed as pub fn in validation.mpl — required for direct import in test files; was private fn in source but plan interface spec listed it as pub
- [Phase 141]: meshc test pure function pattern established: test files in mesher/tests/ import from Ingestion.* modules; Result variants tested via case expression
- [Phase 141 Plan 03]: meshc test project_dir must walk upward from test file path to find nearest main.mpl — using CWD caused entire workspace to be copied to temp build dir
- [Phase 142]: HTTP Client docs use lowercase Http.* for v14 fluent builder and uppercase HTTP.* for legacy single-call API — critical naming distinction documented explicitly
- [Phase 142]: meshpkg section placed as its own H2 alongside Test Runner rather than under Package Manager — separate binary warrants top-level section
- [Phase 142]: Combined Crypto, Base64, Hex, DateTime into single Standard Library page per research recommendation
- [Phase 142]: All doc code examples sourced exclusively from passing e2e tests (no invented syntax)
- [Phase 142]: Testing knowledge captured only in Ecosystem Overview prose — no skills/testing sub-skill file exists or was created in this phase
- [Phase 142]: Http.* (lowercase) = HTTP CLIENT vs HTTP.* (uppercase) = HTTP SERVER distinction documented explicitly in skill files as rule 10 of v14 section
- [Phase 143]: vite ^6 required for @sveltejs/vite-plugin-svelte 6.x peer dep compatibility
- [Phase 143]: type:module required in SvelteKit package.json — kit is ESM-only
- [Phase 143]: vite.config.js required in SvelteKit projects — vite build needs sveltekit() plugin registered
- [Phase 143]: with_secure(true) correct for Fly.io — TLS terminates at edge proxy, HTTPS-only session cookies work correctly for production
- [Phase 143]: cargo-chef multi-stage Dockerfile used for registry — dependency layer caching reduces rebuild times; binary target is mesh-registry
- [Phase 143]: build-meshpkg is a separate CI job (not a matrix entry in build) to avoid 15+ min LLVM cost per runner for a pure-Rust binary
- [Phase 143]: install_binary() helper in install.sh centralizes download/verify/install logic for any mesh toolchain binary by name
- [Phase 143 Plan 01]: ubuntu:24.04 required as Dockerfile runtime base — cargo-chef rust:1 builder produces GLIBC 2.38 binary; debian:bookworm-slim only provides GLIBC 2.35 causing startup linker error
- [Phase 143 Plan 01]: auto_stop_machines must be string 'off' not boolean false in fly.toml — fly launch generates canonical string format; boolean causes fly deploy parse error
- [Phase 143 Plan 04]: packages-website deployed to Fly.io (mesh-packages, iad) instead of Cloudflare Pages — user preference; adapter-node + Dockerfile pattern used; consistent with registry platform
- [Phase 144 Plan 01]: cancel-in-progress: false on deploy-fly concurrency group — never cancel a mid-flight Fly.io deploy (leaves app broken)
- [Phase 144 Plan 01]: flyctl deploy --remote-only required — remote builder handles Docker; Actions runner has no production Docker daemon configured
- [Phase 144 Plan 01]: Per-job working-directory (registry/ vs packages-website/) so flyctl picks up correct fly.toml
- [Phase 144 Plan 01]: curl --retry 5 --retry-delay 10 --retry-connrefused for Fly.io health checks (VM warmup window); --retry 3 --retry-delay 5 for GitHub Pages docs (faster propagation)
- [Phase 145-01]: tailwindcss() placed before sveltekit() in vite plugins — reversed order causes CSS classes to have no effect
- [Phase 145-01]: versions route registered before /{name}/{version} in mod.rs — prevents Axum matching literal 'versions' as a version parameter
- [Phase 145-01]: Tailwind v4 requires no tailwind.config.js — all configuration via CSS @theme inline block

### Roadmap Evolution

- Phase 141 added: Dogfeed v14 changes to mesher
- Phase 142 added: Update docs page with changes/additions from v14
- Phase 143 added: Deploy everything including new stuff from v14
- Phase 144 added: Update GitHub Actions to deploy all services on release branch push with post-deploy health checks
- Phase 145 added: Completely redesign the mesh packages page. It is extremely bare and simple. It should be production grade and look good and fit the theme of the landing page

### Pending Todos

None.

### Blockers/Concerns

- [Phase 138 Plan 01]: Coverage (TEST-10) stub shipped in Plan 01 — --coverage flag accepted, prints "Coverage reporting coming soon", exits 0; full MIR counter injection deferred to v14.1
- [Phase 140]: Registry storage abstraction (StorageBackend trait for S3/R2 migration path) needs design decision at planning time
- [Phase 140]: Empty registry at launch ("ghost town" problem) — plan to publish stdlib packages as seed content during Phase 140

## Session Continuity

Last session: 2026-03-01
Stopped at: Completed 145-01-PLAN.md — Tailwind v4 foundation + OKLCH tokens installed into packages-website; search API bug fixed (?search= param); registry versions endpoint added at GET /api/v1/packages/{name}/versions; npm run build and cargo check both pass
Resume file: None
