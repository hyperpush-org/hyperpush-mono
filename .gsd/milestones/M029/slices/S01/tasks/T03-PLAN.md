---
estimated_steps: 4
estimated_files: 6
skills_used:
  - test
  - review
---

# T03: Repair reference-backend imports and prove round-trip cleanliness

**Slice:** S01 — Formatter dot-path and multiline import fix
**Milestone:** M029

## Description

Move `reference-backend/` out of its corrupted-but-stable state. This task rewrites the real dogfood sources to canonical dotted imports and proves the fixed formatter now preserves them through a full round trip.

## Steps

1. Update the spaced module imports in `reference-backend/main.mpl`, `reference-backend/api/health.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/router.mpl`, `reference-backend/api/jobs.mpl`, and `reference-backend/jobs/worker.mpl` to canonical dotted paths.
2. Preserve the multiline parenthesized import shape in `reference-backend/api/health.mpl` while normalizing the rest of the affected backend files with the fixed formatter.
3. Run `cargo run -q -p meshc -- fmt reference-backend` followed by `cargo run -q -p meshc -- fmt --check reference-backend` and inspect any remaining dot-spacing output before accepting the result.
4. Keep scope tight: stop at dotted-path repair and formatter cleanliness; mesher cleanup and broader import conversion stay in S02/S03.

## Must-Haves

- [ ] All six affected backend imports are restored to canonical `Foo.Bar` form.
- [ ] `reference-backend/api/health.mpl` remains parenthesized and multiline after formatting.
- [ ] `reference-backend/` round-trips through `meshc fmt --check` without reintroducing dot spaces.

## Verification

- `cargo run -q -p meshc -- fmt reference-backend && cargo run -q -p meshc -- fmt --check reference-backend`
- `! rg -n "^from .*\\. " reference-backend -g '*.mpl'`

## Inputs

- `compiler/meshc/tests/e2e_fmt.rs` — CLI formatter proof shape from T02
- `reference-backend/main.mpl` — currently corrupted import path
- `reference-backend/api/health.mpl` — multiline import smoke target
- `reference-backend/storage/jobs.mpl` — currently corrupted import path
- `reference-backend/api/router.mpl` — currently corrupted import path
- `reference-backend/api/jobs.mpl` — currently corrupted import path
- `reference-backend/jobs/worker.mpl` — currently corrupted import path

## Expected Output

- `reference-backend/main.mpl` — canonical dotted imports
- `reference-backend/api/health.mpl` — canonical dotted multiline import
- `reference-backend/storage/jobs.mpl` — canonical dotted imports
- `reference-backend/api/router.mpl` — canonical dotted imports
- `reference-backend/api/jobs.mpl` — canonical dotted imports
- `reference-backend/jobs/worker.mpl` — canonical dotted imports

## Observability Impact

- The primary inspection surface stays textual: `cargo run -q -p meshc -- fmt reference-backend && cargo run -q -p meshc -- fmt --check reference-backend` must show a clean round-trip, and any regression should surface as a formatter diff instead of a silent semantic rewrite.
- The cheap corruption detector remains `! rg -n "^from .*\\. " reference-backend -g '*.mpl'`; if this task regresses, it immediately pinpoints any backend file that still contains spaced dotted imports.
- `reference-backend/api/health.mpl` remains the multiline import smoke target, so future agents can inspect that file directly when checking whether parenthesized dotted imports stayed multiline after a formatter change.
