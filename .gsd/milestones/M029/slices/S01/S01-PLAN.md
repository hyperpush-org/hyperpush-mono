# S01: Formatter dot-path and multiline import fix

**Goal:** Make `meshc fmt` format module `PATH` nodes without inserting dot-path spaces, preserve parenthesized multiline imports with dotted module paths intact, and leave `reference-backend/` in a semantically correct round-trippable state.
**Demo:** New formatter unit and CLI regressions prove `from Api.Router import (...)` and `impl Foo.Bar for Baz.Qux do` keep dotted paths, and `meshc fmt --check reference-backend` passes after the backend imports are repaired to canonical `Foo.Bar` form.

## Must-Haves

- `SyntaxKind::PATH` no longer formats dotted module paths as `Foo. Bar` in imports or qualified impl headers.
- Parenthesized multiline imports keep their multiline shape and preserve dotted module paths exactly.
- Exact-output regressions land in `compiler/mesh-fmt/src/walker.rs`, `compiler/mesh-fmt/src/lib.rs`, and `compiler/meshc/tests/e2e_fmt.rs` so semantically wrong but idempotent output is caught.
- `reference-backend/` source files are rewritten back to canonical dotted imports and stay clean under `meshc fmt --check`.

## Proof Level

- This slice proves: contract
- Real runtime required: no
- Human/UAT required: no

## Verification

- `cargo test -q -p mesh-fmt --lib`
- `cargo test -q -p meshc --test e2e_fmt -- --nocapture`
- `cargo test -q -p meshc --test e2e_fmt fmt_preserves_dotted_paths_exactly -- --nocapture`
- `cargo test -q -p meshc --test e2e e2e_multiline_import_paren -- --nocapture`
- `cargo run -q -p meshc -- fmt --check reference-backend`
- `! rg -n "^from .*\\. " reference-backend -g '*.mpl'`
- `cargo test -q -p mesh-fmt --lib walk_path_preserves_dotted_import_and_impl_paths -- --nocapture`

## Observability / Diagnostics

- Primary inspection surface is exact-output formatter coverage in `compiler/mesh-fmt/src/walker.rs`; failures must show expected vs actual text for dotted imports, parenthesized multiline imports, and qualified impl headers.
- The CLI-level exact-output surface is `compiler/meshc/tests/e2e_fmt.rs::fmt_preserves_dotted_paths_exactly`; it must fail with the actual formatted file text if `meshc fmt` ever emits `Api. Router` or `Foo. Bar` while still otherwise succeeding.
- Secondary inspection surface is `meshc fmt --check reference-backend`, whose stderr diff should expose any regression that reintroduces `Foo. Bar` spacing or collapses multiline import layout.
- The repo-level sweep `rg -n "^from .*\\. " reference-backend -g '*.mpl'` is the cheap corruption detector for already-formatted backend files.
- No new runtime logging or secret-bearing output is introduced in this slice; diagnostics must stay limited to formatter test assertions, unified diffs, and source grep results.

## Integration Closure

- Upstream surfaces consumed: `compiler/mesh-parser/src/parser/items.rs`, `compiler/mesh-fmt/src/walker.rs`, `compiler/mesh-fmt/src/lib.rs`, `compiler/meshc/tests/e2e.rs`, `compiler/meshc/tests/e2e_fmt.rs`, `reference-backend/main.mpl`, `reference-backend/api/health.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/router.mpl`, `reference-backend/api/jobs.mpl`, `reference-backend/jobs/worker.mpl`
- New wiring introduced in this slice: `meshc fmt` keeps its existing entrypoint, but `compiler/mesh-fmt/src/walker.rs` must route `PATH` through dedicated dot-aware formatting instead of the generic inline spacer.
- What remains before the milestone is truly usable end-to-end: mesher JSON/pipe cleanup in S02 and broad multiline-import adoption plus final formatter compliance in S03.

## Tasks

- [x] **T01: Route PATH nodes through dot-aware formatter logic** `est:45m`
  - Why: The live corruption comes from `SyntaxKind::PATH` still falling through `walk_tokens_inline`, so dotted module names in imports and impl headers need a localized formatter fix before any dogfood cleanup is trustworthy.
  - Files: `compiler/mesh-fmt/src/walker.rs`, `compiler/mesh-parser/src/parser/items.rs`, `compiler/mesh-fmt/src/lib.rs`
  - Do: Add a dedicated `PATH` formatting path in `compiler/mesh-fmt/src/walker.rs`, keep the generic token spacer unchanged unless a new failing proof forces broader surgery, and add walker-level exact-output regressions for dotted imports and qualified impl headers.
  - Verify: `cargo test -q -p mesh-fmt --lib`
  - Done when: `PATH` formatting keeps `Foo.Bar` exact in imports and impl headers, and the formatter lib suite stays green.
- [x] **T02: Add truthful library and CLI regressions for dotted imports** `est:45m`
  - Why: Existing coverage can stay green on semantically corrupted-but-idempotent output, so the slice needs library-level snapshots/idempotence and CLI text assertions that would fail on `Api. Router` or `Foo. Bar`.
  - Files: `compiler/mesh-fmt/src/lib.rs`, `compiler/meshc/tests/e2e_fmt.rs`, `compiler/meshc/tests/e2e.rs`
  - Do: Extend the formatter library regressions to dotted imports, add CLI temp-file tests that assert exact formatted text for dotted single-line imports, parenthesized multiline imports, and qualified impl headers, and keep the existing multiline-import compiler e2e green.
  - Verify: `cargo test -q -p meshc --test e2e_fmt -- --nocapture && cargo test -q -p meshc --test e2e e2e_multiline_import_paren -- --nocapture`
  - Done when: The higher-level tests would fail on the old dot-spacing bug and still prove multiline imports stay multiline.
- [x] **T03: Repair reference-backend imports and prove round-trip cleanliness** `est:30m`
  - Why: `reference-backend/` is already in a corrupted-but-stable formatter state, and R027 is not satisfied until the real dogfood source is rewritten to canonical dotted imports and survives `meshc fmt --check`.
  - Files: `reference-backend/main.mpl`, `reference-backend/api/health.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/router.mpl`, `reference-backend/api/jobs.mpl`, `reference-backend/jobs/worker.mpl`
  - Do: Restore the six affected backend imports to canonical `Foo.Bar` form, preserve the multiline parenthesized import shape in `api/health.mpl`, run the fixed formatter on `reference-backend/`, and stop if the work starts drifting into mesher cleanup or unrelated backend refactors.
  - Verify: `cargo run -q -p meshc -- fmt reference-backend && cargo run -q -p meshc -- fmt --check reference-backend && ! rg -n "^from .*\\. " reference-backend -g '*.mpl'`
  - Done when: All affected backend import lines are canonical, `api/health.mpl` stays multiline, and `reference-backend/` round-trips cleanly through `meshc fmt --check`.

## Files Likely Touched

- `compiler/mesh-fmt/src/walker.rs`
- `compiler/mesh-fmt/src/lib.rs`
- `compiler/meshc/tests/e2e_fmt.rs`
- `reference-backend/main.mpl`
- `reference-backend/api/health.mpl`
- `reference-backend/storage/jobs.mpl`
- `reference-backend/api/router.mpl`
- `reference-backend/api/jobs.mpl`
- `reference-backend/jobs/worker.mpl`
