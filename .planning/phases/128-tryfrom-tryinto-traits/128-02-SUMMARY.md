---
phase: 128-tryfrom-tryinto-traits
plan: 02
subsystem: compiler
tags: [rust, codegen, mir, tryfrom, tryinto, e2e, type-checking]

# Dependency graph
requires:
  - phase: 128-01
    provides: TryFrom/TryInto trait registration and synthetic TryInto derivation
  - phase: 77-from-into-traits
    provides: From/Into dispatch pattern in lower.rs used as template

provides:
  - StructName.try_from() static dispatch resolves to TryFrom_X__try_from__StructName in MIR
  - .try_into() instance dispatch redirects to underlying TryFrom function in known_functions
  - 4 passing E2E tests verifying TRYFROM-01/02/03 end-to-end
  - Bug fix: impl method return types now use full generic resolution (Result<T,E>, Option<T>)
  - Bug fix: struct values <= 8 bytes in variant fields now always pointer-boxed (prevents SIGSEGV)
  - Bug fix: synthetic TryInto return type carries the TryFrom return type for type-checker acceptance
affects:
  - Phase 129: Map.collect and Quality - benefits from correct impl return type resolution
  - Any future trait with Result<T,E> return types in impl methods

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Static trait dispatch pattern: scan known_functions for TryFrom_*__try_from__{StructName} mirroring From_*__from__{StructName}"
    - "Instance trait dispatch pattern: when mangled name starts with TryInto__try_into__, redirect to TryFrom_<Source>__try_from__<Target> in known_functions"
    - "Struct boxing invariant: ALL struct values in variant fields must be pointer-boxed regardless of size (ptr slot is always treated as pointer)"

key-files:
  created:
    - tests/e2e/tryfrom_user_defined.mpl
    - tests/e2e/tryfrom_err_path.mpl
    - tests/e2e/tryfrom_try_operator.mpl
    - tests/e2e/tryinto_dispatch.mpl
  modified:
    - compiler/mesh-codegen/src/mir/lower.rs
    - compiler/mesh-codegen/src/codegen/expr.rs
    - compiler/mesh-typeck/src/infer.rs
    - compiler/mesh-typeck/src/traits.rs
    - compiler/meshc/tests/e2e.rs

key-decisions:
  - "Struct boxing threshold changed from >8 to always-box: the ptr slot in {i8,ptr} variant layout is ALWAYS dereferenced as a pointer by pattern matching code, so even 8-byte structs must be heap-allocated"
  - "TryInto return type set to mirror TryFrom return type at synthesis time rather than None, so resolve_trait_method returns Some(ret) and type-checker accepts .try_into() calls"
  - "impl method return type now uses resolve_type_annotation (full generic) over resolve_type_name (simple name only) to handle Result<T,E> and Option<T> in impl bodies"
  - "Fixture syntax uses case (not match) and let r = expr; case r do (not case expr() do) to match parser capabilities"

patterns-established:
  - "Always-box structs in variant fields: codegen_construct_variant boxes ALL struct values, not just those >8 bytes"
  - "Impl return type resolution: resolve_type_annotation preferred over resolve_type_name for generic types"

requirements-completed: [TRYFROM-01, TRYFROM-02, TRYFROM-03]

# Metrics
duration: 22min
completed: 2026-02-27
---

# Phase 128 Plan 02: TryFrom/TryInto E2E Tests Summary

**TryFrom.try_from() and .try_into() wired end-to-end through MIR codegen with 4 passing E2E tests, plus 3 auto-fixed latent bugs in struct boxing, impl return type resolution, and synthetic TryInto type propagation**

## Performance

- **Duration:** ~22 min
- **Started:** 2026-02-27T22:43:40Z
- **Completed:** 2026-02-27T23:05:12Z
- **Tasks:** 2
- **Files modified:** 9 (5 compiler files + 4 fixture files)

## Accomplishments

- StructName.try_from() dispatch in lower_field_access finds TryFrom_*__try_from__{StructName} in known_functions (mirrors existing From.from() pattern)
- .try_into() instance dispatch in resolve_trait_callee redirects TryInto__try_into__<Source> to TryFrom_<Source>__try_from__<Target> in known_functions (synthetic TryInto impls are not in known_functions so this redirection is necessary)
- BUILTIN_PREFIXES in qualify_name extended with From_/From__/Into_/Into__/TryFrom_/TryFrom__/TryInto_/TryInto__ so mangled trait method names are never accidentally module-prefixed
- All 4 E2E tests pass: tryfrom_user_defined (42), tryfrom_err_path (must be positive), tryfrom_try_operator (42/must be positive), tryinto_dispatch (42/must be positive)
- 8 existing From E2E tests pass with zero regressions
- All 248+ mesh-typeck unit tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire TryFrom static dispatch and TryInto instance dispatch in MIR lower.rs** - `3bec8140` (feat)
2. **Task 2: Add E2E fixtures and test functions for TRYFROM-01/02/03** - `7542d03d` (feat)

## Files Created/Modified

- `compiler/mesh-codegen/src/mir/lower.rs` - Added try_from static dispatch in lower_field_access, try_into->TryFrom redirect in resolve_trait_callee, extended BUILTIN_PREFIXES
- `compiler/mesh-codegen/src/codegen/expr.rs` - Fixed struct boxing: always heap-allocate struct values in variant fields (was only boxing for >8 bytes)
- `compiler/mesh-typeck/src/infer.rs` - Fixed impl method return type resolution: use resolve_type_annotation instead of resolve_type_name for generic types like Result<T,E>
- `compiler/mesh-typeck/src/traits.rs` - Fixed synthetic TryInto: capture TryFrom.try_from return type before impl_def move, use it as TryInto.try_into return type
- `compiler/meshc/tests/e2e.rs` - Added 4 E2E test functions (e2e_tryfrom_user_defined, e2e_tryfrom_err_path, e2e_tryfrom_try_operator, e2e_tryinto_dispatch)
- `tests/e2e/tryfrom_user_defined.mpl` - TRYFROM-01 success path
- `tests/e2e/tryfrom_err_path.mpl` - TRYFROM-01 error path
- `tests/e2e/tryfrom_try_operator.mpl` - TRYFROM-03 ? operator propagation
- `tests/e2e/tryinto_dispatch.mpl` - TRYFROM-02 synthetic TryInto dispatch

## Decisions Made

- Struct boxing threshold: changed from `struct_size > 8` to always-box. Reasoning: the variant layout `{i8 tag, ptr data}` always treats the data slot as a pointer during pattern matching (deref). Even an 8-byte struct value stored directly in the ptr slot causes SIGSEGV because the match arm tries to dereference it as a pointer to the struct.
- TryInto return type: set to mirror the TryFrom return type at synthesis time, so `resolve_trait_method("try_into", Int)` returns `Some(Result<PositiveInt, String>)` rather than `None`, allowing the type checker to accept `.try_into()` calls.
- Fixture syntax: use `case` (not `match`) and bind results with `let r = expr` before `case r do` (not `case expr() do`) to match parser capabilities. Use `T!E` syntax for function return types (not `Result<T,E>`) when possible for cleaner syntax.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed impl method return type resolution in infer_impl_def**
- **Found during:** Task 2 (running tryfrom_user_defined E2E test)
- **Issue:** `resolve_type_name` only extracts the top-level IDENT from a type annotation. For `Result<PositiveInt, String>`, it returned bare `Ty::Con("Result")` without type parameters. The type checker then unified this against the inferred body type `Result<PositiveInt, String>` and reported a mismatch.
- **Fix:** Changed `or_else(|| resolve_type_name(&ann))` to `or_else(|| resolve_type_annotation(ctx, &ann, type_registry)).or_else(|| resolve_type_name(&ann))` in the return type resolution path. `resolve_type_annotation` calls `parse_type_tokens` which handles generic type arguments.
- **Files modified:** `compiler/mesh-typeck/src/infer.rs`
- **Verification:** tryfrom_user_defined compiles without type errors
- **Committed in:** `7542d03d` (Task 2 commit)

**2. [Rule 1 - Bug] Fixed struct boxing threshold in codegen_construct_variant**
- **Found during:** Task 2 (running compiled binary -- exit 139 SIGSEGV)
- **Issue:** `codegen_construct_variant` only heap-allocated struct values when `struct_size > 8`. A `PositiveInt { value :: Int }` struct is exactly 8 bytes (single i64), so the condition was false and the struct was stored directly in the ptr slot. Pattern matching code (at `navigate_access_path`) always treats the ptr slot as a pointer and dereferences it -- getting raw value 42 instead of a valid memory address caused SIGSEGV.
- **Fix:** Changed condition from `if struct_size > 8` to always box (removed size check). ALL struct values in variant fields must be pointer-boxed since the deref is unconditional.
- **Files modified:** `compiler/mesh-codegen/src/codegen/expr.rs`
- **Verification:** tryfrom_user_defined binary runs and outputs "42"; from tests all pass
- **Committed in:** `7542d03d` (Task 2 commit)

**3. [Rule 1 - Bug] Fixed synthetic TryInto return type propagation in traits.rs**
- **Found during:** Task 2 (tryinto_dispatch type-check error "no method try_into on type Int")
- **Issue:** Synthetic TryInto synthesis set `return_type: None` in the ImplMethodSig. `resolve_trait_method` returns `None` when `method_sig.return_type` is `None`, which the type checker interprets as "method not found" and emits `NoSuchMethod` error.
- **Fix:** Before moving `impl_def`, capture `impl_def.methods.get("try_from").and_then(|sig| sig.return_type.clone())` as `synth_try_return_ty`. Use this as the `return_type` in the synthesized TryInto ImplMethodSig. The TryFrom.try_from return type `Result<PositiveInt, String>` becomes the TryInto.try_into return type (semantically correct since both convert the same types).
- **Files modified:** `compiler/mesh-typeck/src/traits.rs`
- **Verification:** tryinto_dispatch compiles without type errors and outputs "42\nmust be positive"
- **Committed in:** `7542d03d` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (3x Rule 1 - Bugs)
**Impact on plan:** All 3 bugs were latent issues exposed by TryFrom being the first user-defined trait with Result<Struct,E> return types in impl methods. The fixes are general improvements benefiting all similar patterns.

## Issues Encountered

- Parser limitation discovered: `case expr() do` (inline call) doesn't parse; must use `let r = expr(); case r do`. This is a pre-existing parser behavior, not a regression.
- Parser limitation: `match` keyword not recognized; must use `case`. Updated fixtures accordingly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- TRYFROM-01, TRYFROM-02, TRYFROM-03 all verified end-to-end
- Phase 128 complete: TryFrom/TryInto trait definitions + codegen wiring + E2E tests all passing
- Ready for Phase 129: Map.collect + Quality improvements
- The struct boxing fix and impl return type fix are general improvements that may prevent similar issues in Phase 129

## Self-Check: PASSED
- SUMMARY.md: FOUND
- Commit 3bec8140 (feat: wire TryFrom/TryInto dispatch): FOUND
- Commit 7542d03d (feat: E2E fixtures + 3 bug fixes): FOUND
- tryfrom_user_defined.mpl: FOUND
- tryinto_dispatch.mpl: FOUND

---
*Phase: 128-tryfrom-tryinto-traits*
*Completed: 2026-02-27*
