# S01: Onboarding Graph & Retained Rail Reset

**Goal:** Reset the structural public docs graph by changing navigation sequencing and retargeting the active retained docs-contract rails so the scaffold/examples-first path can ship without fighting legacy proof-first expectations.
**Demo:** After this: On a fresh docs build, the default public path moves through Getting Started and Clustered Example before any proof pages, and updated retained docs contracts fail closed if proof surfaces regain primary-path prominence.

## Tasks
- [x] **T01: Reordered the public docs graph and fixed VitePress footer resolution so Clustered Example no longer self-links and proof pages stay out of the footer chain** — **Slice:** S01 — Onboarding Graph & Retained Rail Reset
**Milestone:** M050

## Description

The real public docs graph lives in `website/docs/.vitepress/config.mts` and `website/docs/.vitepress/theme/composables/usePrevNext.ts`, not just in page prose. Today the proof pages occupy first-contact sidebar slots, and the current footer matcher treats `/docs/getting-started/` as active for `/docs/getting-started/clustered-example/`, which produces a self-linking `Next` footer on Clustered Example. This task changes the actual navigation path first so later copy rewrites are not fighting the wrong structure.

## Steps

1. Move `Production Backend Proof` and `Distributed Proof` out of the primary Getting Started / Distribution groups into a dedicated secondary proof-surface position in the `/docs/` sidebar.
2. Change the prev/next resolver to use exact current-page matching for footer candidates so `Clustered Example` stops resolving through the `Getting Started` prefix.
3. Add `prev: false` and `next: false` frontmatter on the two proof pages so they stay public-secondary without rejoining the footer chain.
4. Add a Node docs-graph contract test that fails closed if proof pages move back into the primary path, if footer matching regresses, or if proof pages lose their footer opt-out.

## Must-Haves

- [ ] Sidebar order makes `Getting Started` and `Clustered Example` the only first-contact Getting Started entries.
- [ ] `Clustered Example` no longer renders a self-linking `Next` footer.
- [ ] Proof pages remain public by URL/sidebar but opt out of prev/next chaining.
- [ ] The new source-level graph contract fails closed on sidebar order, footer matching, and proof-page opt-out regressions.
  - Estimate: 1h30m
  - Files: website/docs/.vitepress/config.mts, website/docs/.vitepress/theme/composables/usePrevNext.ts, website/docs/docs/production-backend-proof/index.md, website/docs/docs/distributed-proof/index.md, scripts/tests/verify-m050-s01-onboarding-graph.test.mjs
  - Verify: - `node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`
- [ ] **T02: Retarget retained docs rails to the secondary-proof contract** — **Slice:** S01 — Onboarding Graph & Retained Rail Reset
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
  - Estimate: 2h
  - Files: compiler/meshc/tests/e2e_m047_s04.rs, compiler/meshc/tests/e2e_m047_s06.rs, scripts/verify-m047-s04.sh, scripts/verify-m047-s06.sh, reference-backend/scripts/verify-production-proof-surface.sh
  - Verify: - `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`
- `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`
- `bash reference-backend/scripts/verify-production-proof-surface.sh`
- [ ] **T03: Add the M050 graph verifier and wire it into the retained wrapper contracts** — **Slice:** S01 — Onboarding Graph & Retained Rail Reset
**Milestone:** M050

## Description

After the live graph and retained contracts are reset, the slice needs one fast named verifier that future tasks can run before deeper example/runtime proofs. This task adds that rail, pins its retained bundle contract, and teaches the active M049 wrapper sources to acknowledge it as the docs-graph preflight.

## Steps

1. Add `scripts/verify-m050-s01.sh` with the standard status/current-phase/phase-report/full-contract.log/latest-proof-bundle pattern; run the new Node graph contract, the retargeted M047 docs contracts, the production proof-surface verifier, a VitePress build, and a built-HTML assertion pass.
2. Add `compiler/meshc/tests/e2e_m050_s01.rs` to pin the new script's phase markers, retained artifact paths, and built-site bundle expectations.
3. Update `scripts/verify-m049-s05.sh`, `scripts/tests/verify-m049-s05-contract.test.mjs`, and `compiler/meshc/tests/e2e_m049_s05.rs` so the active M049 wrapper acknowledges `bash scripts/verify-m050-s01.sh` as the fast docs-graph preflight before heavier example/runtime replays.
4. Keep the M050 verifier fast and env-free: it must not require Postgres or the M049 sample replays to prove the structural docs graph.

## Must-Haves

- [ ] `bash scripts/verify-m050-s01.sh` is the slice-local source+build rail for the onboarding graph reset and emits standard `.tmp/m050-s01/verify` artifacts.
- [ ] Built-site assertions prove `Getting Started → Clustered Example` footer flow, no `Clustered Example` self-loop, and no footer on proof pages.
- [ ] `compiler/meshc/tests/e2e_m050_s01.rs` fails closed when phase markers, retained bundle shape, or built-HTML evidence drift.
- [ ] Active M049 wrapper source contracts acknowledge the new M050 preflight without dragging this slice into Postgres/runtime sample requirements.
  - Estimate: 2h
  - Files: scripts/verify-m050-s01.sh, compiler/meshc/tests/e2e_m050_s01.rs, scripts/verify-m049-s05.sh, scripts/tests/verify-m049-s05-contract.test.mjs, compiler/meshc/tests/e2e_m049_s05.rs
  - Verify: - `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`
- `node --test scripts/tests/verify-m049-s05-contract.test.mjs`
- `cargo test -p meshc --test e2e_m049_s05 -- --nocapture`
- `bash scripts/verify-m050-s01.sh`
