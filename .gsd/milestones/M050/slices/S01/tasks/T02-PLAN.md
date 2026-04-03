---
estimated_steps: 14
estimated_files: 5
skills_used:
  - bash-scripting
  - test
---

# T02: Retarget retained docs rails to the secondary-proof contract

**Slice:** S01 — Onboarding Graph & Retained Rail Reset
**Milestone:** M050

## Description

The active retained docs rails are still exact-string contracts around M047 proof-heavy wording. S02 and S03 need those rails to guard the example/readme split and proof-page discoverability without freezing old intro paragraphs in place. This task repoints the retained M047 and production-proof contracts at the M050 structural graph instead of the current proof-maze copy.

## Steps

1. Rewrite the M047 S04 and S06 docs-contract assertions away from proof-first first-contact wording and toward secondary proof discoverability, example/readme truth, and the new onboarding graph.
2. Update `scripts/verify-m047-s04.sh` and `scripts/verify-m047-s06.sh` so they assert the new M050 structural markers before their retained rails continue.
3. Retarget `reference-backend/scripts/verify-production-proof-surface.sh` so it proves `Production Backend Proof` stays public-secondary instead of assuming Getting Started prominence.
4. Preserve the retained example/readme split and proof-rail discoverability so later copy tasks can rewrite prose without reviving old proof-first strings just to satisfy historical rails.

## Must-Haves

- [ ] Retained M047 docs contracts stop requiring proof-heavy first-contact copy in `Clustered Example`, `Tooling`, `Distributed Actors`, and `Distributed Proof`.
- [ ] Retained verifiers assert proof-page discoverability and demotion instead of sidebar primacy.
- [ ] The production proof-surface verifier still keeps `Production Backend Proof` public, but no longer treats it as a coequal first-contact step.
- [ ] Later M050 copy slices can rewrite prose without restoring stale proof-maze wording just to keep historical rails green.

## Inputs

- `compiler/meshc/tests/e2e_m047_s04.rs`
- `compiler/meshc/tests/e2e_m047_s06.rs`
- `scripts/verify-m047-s04.sh`
- `scripts/verify-m047-s06.sh`
- `reference-backend/scripts/verify-production-proof-surface.sh`
- `scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`
- `website/docs/.vitepress/config.mts`
- `website/docs/.vitepress/theme/composables/usePrevNext.ts`

## Expected Output

- `compiler/meshc/tests/e2e_m047_s04.rs`
- `compiler/meshc/tests/e2e_m047_s06.rs`
- `scripts/verify-m047-s04.sh`
- `scripts/verify-m047-s06.sh`
- `reference-backend/scripts/verify-production-proof-surface.sh`

## Verification

- `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`
- `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`
- `bash reference-backend/scripts/verify-production-proof-surface.sh`

## Observability Impact

- Signals added/changed: retained docs verifiers fail with graph/demotion-specific markers instead of stale paragraph mismatches.
- How a future agent inspects this: read the named Rust docs-contract tests, shell verifier phase logs, and the production-proof surface verifier output.
- Failure state exposed: whether drift is in proof discoverability, graph order, or the example/readme split.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| retained M047 docs-contract tests (`compiler/meshc/tests/e2e_m047_s04.rs`, `compiler/meshc/tests/e2e_m047_s06.rs`) | Stop on the failing retained contract and point at the stale assumption instead of weakening the whole rail. | Preserve the failing test log and do not claim the docs contract migrated. | Treat missing graph markers or missing example/readme anchors as retained-truth drift. |
| retained shell verifiers (`scripts/verify-m047-s04.sh`, `scripts/verify-m047-s06.sh`) | Fail closed on the named phase and keep the existing phase-report/status contract intact. | Preserve the partial verify directory and phase marker for diagnosis. | Reject missing phase markers, bad bundle pointers, or missing retained files as verifier drift. |
| `reference-backend/scripts/verify-production-proof-surface.sh` | Stop when the production proof page stops being publicly discoverable or secondary in the intended way. | Preserve the failing proof-surface log rather than silently skipping the check. | Reject malformed sidebar/group or link assumptions instead of accepting any proof-page placement. |

## Load Profile

- **Shared resources**: retained `m047` verify directories, docs source files shared with later M050 slices, and VitePress build output when wrappers are replayed.
- **Per-operation cost**: source-level Rust contract tests are cheap; the expensive operations are retained shell wrappers that may rebuild docs and copy retained bundles.
- **10x breakpoint**: repeated docs builds and retained artifact copying dominate first, so the task should keep heavy wrapper replays scoped to the rails it actually changes.

## Negative Tests

- **Malformed inputs**: stale exact-string markers that still assume proof-first copy, missing proof-group markers, or missing example/readme anchors.
- **Error paths**: retained phase reports lose a required pass marker, the production proof-surface script still expects first-contact prominence, or a later copy slice cannot change wording without reviving old proof strings.
- **Boundary conditions**: proof pages stay public, the example/readme split remains unchanged, historical rail aliases keep working, and the new graph contract becomes authoritative without deleting retained proof rails.
