---
phase: 117-string-interpolation-heredocs
plan: "01"
subsystem: compiler
tags: [lexer, string-interpolation, mesh-lexer, e2e]

requires:
  - phase: 116-slot-pipe-operator
    provides: stable compiler foundation for adding lexer features

provides:
  - "#{expr} interpolation in regular strings — lexer emits InterpolationStart for both ${} and #{}"
  - "E2E fixture and test for #{} interpolation (variables, arithmetic, booleans)"
  - "Backward compat: ${expr} continues to work unchanged"

affects:
  - 117-02 (heredoc interpolation — same lexer state machine)
  - 118 (any further string feature work)

tech-stack:
  added: []
  patterns:
    - "Dual-delimiter pattern: both ${ and #{ emit identical InterpolationStart tokens, parser/codegen see no difference"
    - "Rust format string escaping: use #{{}} in assert messages to avoid Rust treating #{} as format arg"

key-files:
  created:
    - tests/e2e/string_interp_hash.mpl
  modified:
    - crates/mesh-lexer/src/lib.rs
    - crates/meshc/tests/e2e.rs

key-decisions:
  - "Both ${ and #{ emit identical InterpolationStart tokens — parser/codegen require zero changes"
  - "\\#{ escapes correctly via existing catch-all backslash handler (consumes \\ then #, leaving { as plain content)"
  - "assert_eq! message uses #{{}} Rust escaping to avoid format string parse error"

patterns-established:
  - "Dual-delimiter interpolation: add new syntax by duplicating the match arm with the new sigil"

requirements-completed: [STRG-01]

duration: 8min
completed: 2026-02-25
---

# Phase 117 Plan 01: String Interpolation (#{}) Summary

**#{expr} interpolation added to the lexer via a second match arm — both ${ and #{ emit InterpolationStart, giving zero-change backward compat with all existing ${} tests**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-02-25T00:00:00Z
- **Completed:** 2026-02-25T00:08:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Added `Some('#') if peek_next == Some('{')` match arm to `lex_string_content` in mesh-lexer, emitting `InterpolationStart` — identical to `${` arm
- Created `tests/e2e/string_interp_hash.mpl` exercising variables, integer arithmetic, booleans, and embedded-in-text interpolation
- Added `e2e_string_interp_hash` test function asserting all five println outputs are correct
- All 36 mesh-lexer unit tests pass; both `e2e_string_interp` and `e2e_string_interp_hash` pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Lexer — emit InterpolationStart for #{...}** - `febc9b57` (feat)
2. **Task 2: E2E fixture + test for #{} interpolation** - `ad766520` (feat)

## Files Created/Modified

- `/Users/sn0w/Documents/dev/snow/crates/mesh-lexer/src/lib.rs` - Added `Some('#') + Some('{')` match arm in `lex_string_content` (16 lines)
- `/Users/sn0w/Documents/dev/snow/tests/e2e/string_interp_hash.mpl` - E2E fixture with 5 println statements exercising #{} syntax
- `/Users/sn0w/Documents/dev/snow/crates/meshc/tests/e2e.rs` - Added `e2e_string_interp_hash` test function

## Decisions Made

- Both `${` and `#{` emit identical `InterpolationStart` tokens — the parser and code-generator require zero changes; only the lexer needed updating.
- `\#{` is correctly handled by the existing catch-all backslash arm, which consumes `\` then `#`, leaving `{` as plain string content.
- Rust's `assert_eq!` format string treats `#{}` as a format argument; escaped as `#{{}}` in the message literal.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Rust format string parse error in test message**
- **Found during:** Task 2 (E2E test authoring)
- **Issue:** `assert_eq!` message string `"#{} interpolation..."` caused compile error because Rust treats `{}` inside format strings as a positional argument
- **Fix:** Escaped as `#{{}} interpolation...` using Rust's brace-doubling escape
- **Files modified:** `crates/meshc/tests/e2e.rs`
- **Verification:** Test compiles and passes
- **Committed in:** ad766520 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - minor compile bug in test code)
**Impact on plan:** Trivial fix; no scope change.

## Issues Encountered

None beyond the Rust format string escape noted above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `#{expr}` interpolation is fully operational in regular `"..."` strings
- Plan 02 can proceed with heredoc support (`"""..."""` with `#{expr}` interpolation)
- All existing `${}` E2E tests remain green — no regressions

---
*Phase: 117-string-interpolation-heredocs*
*Completed: 2026-02-25*

## Self-Check: PASSED

- FOUND: crates/mesh-lexer/src/lib.rs
- FOUND: tests/e2e/string_interp_hash.mpl
- FOUND: crates/meshc/tests/e2e.rs
- FOUND: .planning/phases/117-string-interpolation-heredocs/117-01-SUMMARY.md
- FOUND commit: febc9b57 (Task 1)
- FOUND commit: ad766520 (Task 2)
