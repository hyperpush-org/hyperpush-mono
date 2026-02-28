---
phase: 135-encoding-crypto-stdlib
plan: 01
subsystem: stdlib
tags: [crypto, sha256, sha512, hmac, uuid, rust, llvm, typeck]

# Dependency graph
requires:
  - phase: 119-regex
    provides: "Three-file stdlib pattern (runtime extern C, typeck builtins, LLVM intrinsics) used as template"
provides:
  - "Crypto stdlib module: sha256, sha512, hmac_sha256, hmac_sha512, secure_compare, uuid4"
  - "6 extern C functions in mesh-rt/src/crypto.rs"
  - "Type registrations in mesh-typeck builtins.rs and infer.rs"
  - "LLVM intrinsic declarations in mesh-codegen intrinsics.rs"
  - "MIR lowering wiring in mesh-codegen lower.rs"
  - "5 e2e test fixtures and Rust test functions"
affects: ["135-02-encoding", "136-datetime", "138-testing"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Three-file stdlib pattern: runtime extern C (mesh-rt) + typeck registrations (builtins.rs + infer.rs stdlib_modules) + LLVM wiring (intrinsics.rs + lower.rs)"
    - "Four registration points required for module-qualified access: builtins.rs, infer.rs stdlib_modules(), infer.rs STDLIB_MODULE_NAMES, lower.rs STDLIB_MODULES"
    - "Bool-returning extern C functions use i8_type in LLVM intrinsics (not ptr_type)"
    - "String interpolation (dollar{var}) used to print non-String types from e2e fixtures"

key-files:
  created:
    - compiler/mesh-rt/src/crypto.rs
    - tests/e2e/crypto_sha256.mpl
    - tests/e2e/crypto_sha512.mpl
    - tests/e2e/crypto_hmac.mpl
    - tests/e2e/crypto_secure_compare.mpl
    - tests/e2e/crypto_uuid4.mpl
  modified:
    - compiler/mesh-rt/src/lib.rs
    - compiler/mesh-typeck/src/builtins.rs
    - compiler/mesh-typeck/src/infer.rs
    - compiler/mesh-codegen/src/mir/lower.rs
    - compiler/mesh-codegen/src/codegen/intrinsics.rs
    - compiler/meshc/tests/e2e.rs

key-decisions:
  - "HMAC-SHA256 test vector in plan was incorrect (5bdcc...a72840) — corrected to openssl-verified value (5bdcc...ec3843)"
  - "infer.rs has TWO required registration points for stdlib modules: stdlib_modules() HashMap and STDLIB_MODULE_NAMES const — plan only mentioned builtins.rs; both required for Crypto.* qualified access"
  - "e2e fixtures use string interpolation (dollar{var}) for Bool/Int printing since println() only accepts String"

patterns-established:
  - "New stdlib module requires 5 registration points: (1) builtins.rs env.insert, (2) infer.rs stdlib_modules() HashMap, (3) infer.rs STDLIB_MODULE_NAMES const, (4) lower.rs STDLIB_MODULES const + map_builtin_name + known_functions, (5) intrinsics.rs LLVM declarations"

requirements-completed: [CRYPTO-01, CRYPTO-02, CRYPTO-03, CRYPTO-04, CRYPTO-05, CRYPTO-06]

# Metrics
duration: 6min
completed: 2026-02-28
---

# Phase 135 Plan 01: Crypto Stdlib Module Summary

**6-function Crypto stdlib (sha256, sha512, hmac_sha256, hmac_sha512, secure_compare, uuid4) using sha2/hmac/rand crates already in Cargo.toml, constant-time secure_compare, RFC 4122 uuid4, all 5 e2e tests passing**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-28T07:09:20Z
- **Completed:** 2026-02-28T07:15:01Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- Implemented `compiler/mesh-rt/src/crypto.rs` with 6 extern C functions using sha2, hmac, and rand 0.9 crates (already present in Cargo.toml — zero new dependencies)
- Wired Crypto module through all 5 compiler registration points: builtins.rs, infer.rs, lower.rs, intrinsics.rs, and the previously undocumented infer.rs `stdlib_modules()` HashMap
- All 5 e2e tests pass: sha256 matches NIST test vector, uuid4 produces 36-char RFC 4122 v4 format, secure_compare is constant-time via `std::hint::black_box`

## Task Commits

Each task was committed atomically:

1. **Task 1: Create crypto.rs runtime module with all 6 Crypto extern C functions** - `589730aa` (feat)
2. **Task 2: Wire Crypto module into typeck, MIR lowering, and LLVM intrinsics** - `83483bb3` (feat)
3. **Task 3: Write Crypto e2e fixtures and Rust test functions** - `c7fb42f8` (feat)

## Files Created/Modified

- `compiler/mesh-rt/src/crypto.rs` - 6 extern C functions: sha256, sha512, hmac_sha256, hmac_sha512, secure_compare (constant-time), uuid4 (RFC 4122 v4)
- `compiler/mesh-rt/src/lib.rs` - Added `pub mod crypto;`
- `compiler/mesh-typeck/src/builtins.rs` - 6 Crypto function type registrations
- `compiler/mesh-typeck/src/infer.rs` - Crypto module in `stdlib_modules()` HashMap + `STDLIB_MODULE_NAMES` const
- `compiler/mesh-codegen/src/mir/lower.rs` - "Crypto" in STDLIB_MODULES + 6 map_builtin_name entries + 6 known_functions entries
- `compiler/mesh-codegen/src/codegen/intrinsics.rs` - 6 LLVM External declarations (secure_compare uses i8_type)
- `compiler/meshc/tests/e2e.rs` - 5 Rust e2e test functions
- `tests/e2e/crypto_sha256.mpl` - SHA-256 fixture (NIST test vector)
- `tests/e2e/crypto_sha512.mpl` - SHA-512 fixture
- `tests/e2e/crypto_hmac.mpl` - HMAC-SHA256 and HMAC-SHA512 fixtures
- `tests/e2e/crypto_secure_compare.mpl` - Constant-time comparison fixture
- `tests/e2e/crypto_uuid4.mpl` - UUID v4 format length fixture

## Decisions Made

- Used `std::hint::black_box` for secure_compare to prevent LLVM optimizing away the constant-time loop
- UUID v4 uses `rand::rng().fill_bytes()` (rand 0.9 API, not removed `thread_rng`)
- e2e fixtures use `"${var}"` string interpolation to print Bool/Int values (println only accepts String in Mesh)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added Crypto to infer.rs stdlib_modules() and STDLIB_MODULE_NAMES**
- **Found during:** Task 3 (e2e fixtures compilation)
- **Issue:** Plan only specified builtins.rs registration; typechecker also requires `stdlib_modules()` HashMap in infer.rs for module-qualified access (Crypto.sha256) and `STDLIB_MODULE_NAMES` const for is_stdlib_module() check. Without these, Mesh compiler reports "undefined variable: Crypto" at the call site.
- **Fix:** Added Crypto module to both `stdlib_modules()` and `STDLIB_MODULE_NAMES` in `compiler/mesh-typeck/src/infer.rs`
- **Files modified:** compiler/mesh-typeck/src/infer.rs
- **Verification:** All 5 e2e tests pass after the fix
- **Committed in:** c7fb42f8 (Task 3 commit)

**2. [Rule 1 - Bug] Corrected incorrect HMAC-SHA256 test vector**
- **Found during:** Task 3 (e2e_crypto_hmac test failure)
- **Issue:** Plan specified HMAC-SHA256("Jefe", "what do ya want for nothing?") = `5bdcc146...a72840`. OpenSSL verification produces `5bdcc146...ec3843`. The runtime implementation was correct; the plan's expected value was wrong.
- **Fix:** Updated e2e.rs test assertion to use the openssl-verified digest `5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843`
- **Files modified:** compiler/meshc/tests/e2e.rs
- **Verification:** e2e_crypto_hmac passes with corrected value
- **Committed in:** c7fb42f8 (Task 3 commit)

**3. [Rule 1 - Bug] Fixed e2e fixture type errors (Bool/Int printing)**
- **Found during:** Task 3 (secure_compare and uuid4 fixture type errors)
- **Issue:** `println()` accepts only String in Mesh. `secure_compare` returns Bool, `String.length` returns Int. Fixtures needed type-compatible output expressions.
- **Fix:** Used `"${var}"` string interpolation in both fixtures (matches pattern from stdlib_module_qualified.mpl and other existing e2e tests)
- **Files modified:** tests/e2e/crypto_secure_compare.mpl, tests/e2e/crypto_uuid4.mpl
- **Verification:** Both tests pass
- **Committed in:** c7fb42f8 (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (1 missing critical registration, 1 incorrect plan data, 1 fixture type error)
**Impact on plan:** All fixes necessary for correctness. The infer.rs pattern is now documented for future stdlib modules (5 registration points, not 3).

## Issues Encountered

None beyond what is documented in Deviations.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Crypto module fully operational — Mesh programs can call `Crypto.sha256`, `Crypto.hmac_sha256`, `Crypto.uuid4`, etc.
- Plan 02 (Encoding: Base64, Hex) can follow the same 5-registration pattern now documented
- The infer.rs registration pattern is now fully understood for Phase 135 Plan 02 and future stdlib phases

---
*Phase: 135-encoding-crypto-stdlib*
*Completed: 2026-02-28*

## Self-Check: PASSED

All files verified present:
- compiler/mesh-rt/src/crypto.rs: FOUND
- tests/e2e/crypto_sha256.mpl through crypto_uuid4.mpl: FOUND (5 files)
- .planning/phases/135-encoding-crypto-stdlib/135-01-SUMMARY.md: FOUND

All commits verified:
- 589730aa (Task 1: crypto.rs runtime): FOUND
- 83483bb3 (Task 2: compiler wiring): FOUND
- c7fb42f8 (Task 3: fixtures and tests): FOUND
