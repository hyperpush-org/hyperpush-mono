---
phase: 118-env-var-stdlib
verified: 2026-02-25T00:00:00Z
status: passed
score: 8/8 must-haves verified
re_verification: false
---

# Phase 118: Env Var Stdlib Verification Report

**Phase Goal:** Add Env.get(key, default) -> String and Env.get_int(key, default) -> Int stdlib functions, with E2E tests and migration of all existing callers from the old Option-returning single-argument API.
**Verified:** 2026-02-25
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP success criteria + PLAN must_haves)

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Mesh compiler accepts `Env.get(key, default)` with two arguments and returns String | VERIFIED | `env_get_with_default` registered in builtins.rs (line 253) with `Ty::fun(vec![Ty::string(), Ty::string()], Ty::string())`; `env_get` arm in map_builtin_name routes to `mesh_env_get_with_default`; infer.rs stdlib_modules Env entry has 2-arg get |
| 2 | Mesh compiler accepts `Env.get_int(key, default)` with two arguments and returns Int | VERIFIED | `env_get_int` registered in builtins.rs (line 258) with `Ty::fun(vec![Ty::string(), Ty::int()], Ty::int())`; `env_get_int` arm in map_builtin_name routes to `mesh_env_get_int`; infer.rs has `get_int` entry |
| 3 | Mesh compiler accepts `Env.args()` and returns List<String> | VERIFIED | `env_args` registered in builtins.rs (line 263) with `Ty::fun(vec![], Ty::list(Ty::string()))`; infer.rs has `args` entry; map_builtin_name routes to `mesh_env_args` |
| 4 | `cargo build -p meshc` succeeds with zero errors after all changes | VERIFIED | All 4 commits (bb6c55a9, 11260fa5, 9ccfa817, 4cc8c7e2) report clean builds; commit 4cc8c7e2 summary confirms `cargo build -p meshc exits 0` |
| 5 | `Env.get(key, default)` returns env var string value when set, or default when unset | VERIFIED | `mesh_env_get_with_default` in env.rs: `Ok(val)` → new MeshString, `Err(_)` → `default as *mut MeshString`; unit test `test_env_get_with_default_missing` confirms fallback |
| 6 | `Env.get_int(key, default)` returns parsed int when set to numeric string, or default when unset or non-numeric | VERIFIED | `mesh_env_get_int` in env.rs: `Some(val) => val.parse::<i64>().unwrap_or(default)`, `None => default`; unit tests for missing key and non-numeric both pass |
| 7 | E2E tests for STRG-04 and STRG-05 pass | VERIFIED | `e2e_env_get` (line 232 in e2e.rs) and `e2e_env_get_int` (line 250) both present with `compile_and_run_with_env` helper; fixtures `tests/e2e/env_get.mpl` and `tests/e2e/env_get_int.mpl` exist and are substantive (3 and 4 scenarios respectively); summary reports 264 E2E tests pass |
| 8 | All existing callers migrated from 1-arg `Env.get` to 2-arg `Env.get(key, default)` | VERIFIED | `mesher/main.mpl`: 5 occurrences of `Env.get`, all have two arguments; `get_env_or_default` helper absent; `crates/meshc/src/migrate.rs` line 141 shows `Env.get("DATABASE_URL", "")` 2-arg form; grep for 1-arg calls returns zero matches |

**Score:** 8/8 truths verified

---

## Required Artifacts

### Plan 01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/mesh-rt/src/env.rs` | `mesh_env_get_with_default` and `mesh_env_get_int` runtime functions | VERIFIED | Both functions present at lines 31-55; `#[no_mangle] pub extern "C"`; unit tests at lines 118-143 |
| `crates/mesh-codegen/src/codegen/intrinsics.rs` | LLVM intrinsic declarations for new env functions | VERIFIED | Lines 247-253: `mesh_env_get_with_default` (ptr, ptr -> ptr) and `mesh_env_get_int` (ptr, i64 -> i64) declared; test assertions at lines 1388-1389 |
| `crates/mesh-typeck/src/builtins.rs` | Type signatures for `Env.get`, `Env.get_int`, `Env.args`; old bare `env_get` removed | VERIFIED | Lines 252-265: `env_get_with_default`, `env_get_int`, `env_args` all registered; no `"env_get"` bare entry found (only comment at line 251) |
| `crates/mesh-codegen/src/mir/lower.rs` | `map_builtin_name` and `known_functions` entries for new env functions | VERIFIED | `known_functions` lines 647-654: both `mesh_env_get_with_default` and `mesh_env_get_int` with correct MirType signatures; `map_builtin_name` lines 10439-10441: `env_get` routes to `mesh_env_get_with_default`, `env_get_int` and `env_get_with_default` arms present |

### Plan 02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `tests/e2e/env_get.mpl` | Mesh fixture for `Env.get` E2E scenarios | VERIFIED | 8-line fixture; 3 scenarios: missing var (default), set var (value), empty var (empty string); uses `Env.get` correctly |
| `tests/e2e/env_get_int.mpl` | Mesh fixture for `Env.get_int` E2E scenarios | VERIFIED | 10-line fixture; 4 scenarios: missing (8080), valid int (3000), non-numeric (8080), negative (-1); uses string interpolation `"${val}"` correctly |
| `crates/meshc/tests/e2e.rs` | E2E test functions and `compile_and_run_with_env` helper | VERIFIED | `compile_and_run_with_env` at line 51 — full implementation matching plan spec (tempdir, compile, inject env vars via `cmd.env()`, run binary, return stdout); `e2e_env_get` at line 232 asserts `"default_val\nhello\n\n"`; `e2e_env_get_int` at line 250 asserts `"8080\n3000\n8080\n-1\n"` |
| `mesher/main.mpl` | Mesher entry point migrated to `Env.get(key, default)` | VERIFIED | Lines 44, 63, 72, 119, 121 all show 2-arg `Env.get(key, "")` form; `get_env_or_default` helper absent; no 1-arg calls remain |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Env.get` in Mesh source | `mesh_env_get_with_default` in runtime | `STDLIB_MODULES dispatch: env_get -> map_builtin_name -> mesh_env_get_with_default` | WIRED | infer.rs stdlib_modules Env.get (2-arg) -> builtins.rs `env_get_with_default` type entry -> map_builtin_name arm `"env_get" => "mesh_env_get_with_default"` -> intrinsic declaration -> JIT `add_sym` at line 249 -> runtime function at env.rs line 31 |
| `Env.get_int` in Mesh source | `mesh_env_get_int` in runtime | `STDLIB_MODULES dispatch: env_get_int -> map_builtin_name -> mesh_env_get_int` | WIRED | infer.rs stdlib_modules `get_int` -> builtins.rs `env_get_int` type entry -> map_builtin_name arm `"env_get_int" => "mesh_env_get_int"` -> intrinsic declaration -> JIT `add_sym` at line 250 -> runtime function at env.rs line 49 |
| e2e.rs test functions | fixture files | `read_fixture` + `compile_and_run_with_env` | WIRED | `e2e_env_get` calls `read_fixture("env_get.mpl")` then `compile_and_run_with_env`; `e2e_env_get_int` calls `read_fixture("env_get_int.mpl")` then `compile_and_run_with_env` |
| `compile_and_run_with_env` helper | compiled binary | `Command::new(&binary).envs(env_vars).output()` | WIRED | Lines 70-73: iterates `env_vars` calling `cmd.env(key, val)` before running binary; output returned as String |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| STRG-04 | 118-01-PLAN.md, 118-02-PLAN.md | User can read env var with default: `Env.get("KEY", "default") -> String` | SATISFIED | Runtime function `mesh_env_get_with_default` implemented; full compiler pipeline wired; E2E test `e2e_env_get` passes with expected output `"default_val\nhello\n\n"` |
| STRG-05 | 118-01-PLAN.md, 118-02-PLAN.md | User can parse env var as integer with default: `Env.get_int("PORT", 8080) -> Int` | SATISFIED | Runtime function `mesh_env_get_int` implemented with silent fallback on parse failure; full compiler pipeline wired; E2E test `e2e_env_get_int` passes with expected output `"8080\n3000\n8080\n-1\n"` |

No orphaned requirements found. Both STRG-04 and STRG-05 are claimed by both plans and verified by artifacts.

---

## Anti-Patterns Found

No blockers found. Notable observations:

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/mesh-codegen/src/mir/lower.rs` | 638-641 | Old `mesh_env_get` still present in `known_functions` | INFO | The old Option-returning `mesh_env_get` remains registered in `known_functions` and JIT (line 247 in jit.rs). This is harmless — the function exists in the runtime and the type-checker no longer exposes a path to call it from Mesh source. It does not block any goal. |

---

## Human Verification Required

None. All truths are mechanically verifiable from source code and commit history. The E2E tests cover the observable behavior (env var injection, default fallback, int parsing) end-to-end.

---

## Summary

Phase 118 goal is fully achieved. All eight observable truths verified against the actual codebase:

- Runtime functions `mesh_env_get_with_default` and `mesh_env_get_int` are implemented, unit-tested, and exported from `mesh-rt`.
- The full compiler pipeline is wired: LLVM intrinsic declarations, type-checker entries in both `builtins.rs` and `infer.rs stdlib_modules`, `map_builtin_name` routing, `known_functions` MIR entries, and JIT symbol registrations.
- The old bare `env_get` (Option-returning, 1-arg) has been removed from `builtins.rs` — no alias kept.
- E2E fixtures `env_get.mpl` and `env_get_int.mpl` test all required scenarios. The `compile_and_run_with_env` helper correctly injects env vars into the compiled binary.
- All callers migrated: `mesher/main.mpl` has 5 2-arg `Env.get` calls with no 1-arg forms remaining; `get_env_or_default` helper removed; `migrate.rs` embedded template updated.
- Four commits (bb6c55a9, 11260fa5, 9ccfa817, 4cc8c7e2) verified in git history with correct file change sets.

---

_Verified: 2026-02-25_
_Verifier: Claude (gsd-verifier)_
