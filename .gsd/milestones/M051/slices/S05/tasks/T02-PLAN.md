---
estimated_steps: 4
estimated_files: 7
skills_used:
  - vitepress
  - test
  - rust-testing
---

# T02: Remove the last public `reference-backend` wording and tighten the docs contracts

**Slice:** S05 — Delete reference-backend and close the assembled acceptance rail
**Milestone:** M051

## Description

Clean up the remaining public wording that still leaks `reference-backend/` into the examples-first story, and strengthen the docs-side contract rails so those leaks cannot come back unnoticed.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| Public docs markdown | fail closed on the first stale `reference-backend` marker or missing Mesher/retained-verifier marker | N/A for source assertions | treat wrong wording or ordering as a real public-contract regression |
| Docs contract tests | keep each Node/Rust guard aligned with the shipped wording and fail on the first mismatch | use bounded test invocations; do not ignore red source contracts | treat stale exclusions or missing allowed markers as contract drift |
| S04 wrapper expectations | update the source and built-html checks together so `scripts/verify-m051-s04.sh` stays truthful | respect existing wrapper timeouts | treat built-html omissions or stale public markers as a release blocker |

## Load Profile

- **Shared resources**: public docs markdown, docs contract tests, and the `.tmp/m051-s04/verify/` built-html replay.
- **Per-operation cost**: a handful of markdown edits plus three contract-test updates and one S04 verifier contract update.
- **10x breakpoint**: repeated built-html replays and VitePress build checks dominate before source assertions do.

## Negative Tests

- **Malformed inputs**: bare `reference-backend/` mentions, same-file definition examples pinned to `reference-backend/api/jobs.mpl`, or distributed-proof bullets that still call the deleted app the deeper backend surface.
- **Error paths**: a source contract passes locally but `scripts/verify-m051-s04.sh` still expects the old wording in built HTML or copied wrapper output.
- **Boundary conditions**: public docs stay examples-first and generic while maintainer-only Mesher and retained-fixture surfaces remain discoverable only through the proof page.

## Steps

1. Rewrite the stale public wording in `website/docs/docs/tooling/index.md` and `website/docs/docs/distributed-proof/index.md` so the tooling proof is described generically against a backend-shaped project and the distributed proof page no longer names repo-root `reference-backend` as the deeper backend surface.
2. Tighten `scripts/tests/verify-m036-s03-contract.test.mjs`, `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`, and `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` so they fail closed on the exact stale markers this slice is removing.
3. Update `compiler/meshc/tests/e2e_m051_s04.rs` and `scripts/verify-m051-s04.sh` so the existing S04 acceptance surface expects the new wording and top-level proof-page verifier command instead of the deleted backend path.
4. Re-run the docs contracts and the S04 contract target to prove the public wording and built-html checks are aligned before the tree deletion task.

## Must-Haves

- [ ] `website/docs/docs/tooling/index.md` stops naming `reference-backend/` or `reference-backend/api/jobs.mpl` in the public LSP/editor proof story.
- [ ] `website/docs/docs/distributed-proof/index.md` removes the stale `reference-backend` deeper-backend bullet and keeps the public-secondary verifier map truthful.
- [ ] The M036, M050, and M051 docs/source contracts explicitly catch the removed stale markers.
- [ ] `scripts/verify-m051-s04.sh` and `compiler/meshc/tests/e2e_m051_s04.rs` stay green against the post-cleanup wording.

## Verification

- `node --test scripts/tests/verify-m036-s03-contract.test.mjs`
- `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`
- `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
- `cargo test -p meshc --test e2e_m051_s04 -- --nocapture`

## Observability Impact

- Signals added/changed: the existing S04 built-html contract and source-contract errors now name the last public `reference-backend` markers explicitly.
- How a future agent inspects this: start with the three Node contracts, then run `cargo test -p meshc --test e2e_m051_s04 -- --nocapture` if built-html or wrapper expectations still drift.
- Failure state exposed: the failing page, stale marker, or built-html omission is surfaced directly instead of being inferred from prose review.

## Inputs

- `website/docs/docs/tooling/index.md` — current public tooling wording that still leaks repo-root backend paths
- `website/docs/docs/distributed-proof/index.md` — current distributed-proof bullet list with stale backend language
- `scripts/tests/verify-m036-s03-contract.test.mjs` — editor/docs contract that should forbid stale tooling wording
- `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs` — first-contact contract that should keep public examples-first wording honest
- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` — secondary-surface contract that should forbid stale backend proof wording
- `compiler/meshc/tests/e2e_m051_s04.rs` — current S04 contract target with old proof-page expectations
- `scripts/verify-m051-s04.sh` — existing S04 assembled verifier and built-html checks

## Expected Output

- `website/docs/docs/tooling/index.md` — public tooling wording rewritten around a generic backend-shaped proof surface
- `website/docs/docs/distributed-proof/index.md` — public distributed-proof wording cleaned of repo-root backend references
- `scripts/tests/verify-m036-s03-contract.test.mjs` — tightened editor/docs contract exclusions
- `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs` — tightened first-contact docs contract exclusions
- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` — tightened secondary-surface docs contract exclusions
- `compiler/meshc/tests/e2e_m051_s04.rs` — S04 contract updated to the post-cleanup wording
- `scripts/verify-m051-s04.sh` — S04 wrapper/built-html checks updated to the post-cleanup wording
