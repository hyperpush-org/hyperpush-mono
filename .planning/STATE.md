---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: Ecosystem & Standard Library
status: unknown
last_updated: "2026-02-28T23:49:39.511Z"
progress:
  total_phases: 126
  completed_phases: 126
  total_plans: 330
  completed_plans: 330
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-28)

**Core value:** Expressive, readable concurrency -- writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.
**Current focus:** v14.0 Phase 139 — Package Manifest & meshpkg CLI (phase complete, plan 02 done)

## Current Position

Phase: 139 of 140 (Package Manifest & meshpkg CLI) — COMPLETE (2 of 2 plans complete)
Plan: 2 of 2 in current phase — COMPLETE
Status: Phase 139 complete — Phase 140 (Package Registry Backend & Website) is next
Last activity: 2026-03-01 — Phase 139 Plan 02 complete: meshpkg CLI binary with login/publish/install/search subcommands; clap CLI, tarball creation, SHA-256 verification, ureq 3 HTTP, credentials at ~/.mesh/credentials; PKG-03 + PKG-04 + PKG-05 + PKG-06 satisfied; 30 mesh-pkg tests still pass

Progress: [██████████] 92%  (12/13 plans)

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
| 140. Package Registry Backend & Website | 2 | Not started |

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

### Pending Todos

None.

### Blockers/Concerns

- [Phase 138 Plan 01]: Coverage (TEST-10) stub shipped in Plan 01 — --coverage flag accepted, prints "Coverage reporting coming soon", exits 0; full MIR counter injection deferred to v14.1
- [Phase 140]: Registry storage abstraction (StorageBackend trait for S3/R2 migration path) needs design decision at planning time
- [Phase 140]: Empty registry at launch ("ghost town" problem) — plan to publish stdlib packages as seed content during Phase 140

## Session Continuity

Last session: 2026-03-01
Stopped at: Completed 139-02-PLAN.md — meshpkg CLI binary with login/publish/install/search; clap CLI, GzEncoder tarball, SHA-256 verify, ureq 3 HTTP; PKG-03 to PKG-06 satisfied; Phase 139 complete; ready for Phase 140 (Registry Backend & Website)
Resume file: None
