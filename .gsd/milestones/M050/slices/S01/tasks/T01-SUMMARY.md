---
id: T01
parent: S01
milestone: M050
provides: []
requires: []
affects: []
key_files: ["website/docs/.vitepress/config.mts", "website/docs/.vitepress/theme/composables/usePrevNext.ts", "website/docs/docs/production-backend-proof/index.md", "website/docs/docs/distributed-proof/index.md", "scripts/tests/verify-m050-s01-onboarding-graph.test.mjs", ".gsd/KNOWLEDGE.md", ".gsd/DECISIONS.md"]
key_decisions: ["Use exact normalized path equality for docs footer matching instead of prefix-based sidebar activation.", "Keep proof pages public in the sidebar but out of the footer chain with `includeInFooter: false` plus page-level `prev: false` / `next: false`."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Passed `node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`, `npm --prefix website run build`, `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`, `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`, and `bash reference-backend/scripts/verify-production-proof-surface.sh`. Also previewed the built docs locally and confirmed `Clustered Example` renders Previous -> `/docs/getting-started/` and Next -> `/docs/language-basics/`, while both proof pages render zero `/docs/` footer links. As expected for an intermediate slice task, the future-task surfaces `cargo test -p meshc --test e2e_m050_s01 -- --nocapture` and `bash scripts/verify-m050-s01.sh` still fail because they do not exist until T03."
completed_at: 2026-04-03T17:35:48.325Z
blocker_discovered: false
---

# T01: Reordered the public docs graph and fixed VitePress footer resolution so Clustered Example no longer self-links and proof pages stay out of the footer chain

> Reordered the public docs graph and fixed VitePress footer resolution so Clustered Example no longer self-links and proof pages stay out of the footer chain

## What Happened
---
id: T01
parent: S01
milestone: M050
key_files:
  - website/docs/.vitepress/config.mts
  - website/docs/.vitepress/theme/composables/usePrevNext.ts
  - website/docs/docs/production-backend-proof/index.md
  - website/docs/docs/distributed-proof/index.md
  - scripts/tests/verify-m050-s01-onboarding-graph.test.mjs
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
key_decisions:
  - Use exact normalized path equality for docs footer matching instead of prefix-based sidebar activation.
  - Keep proof pages public in the sidebar but out of the footer chain with `includeInFooter: false` plus page-level `prev: false` / `next: false`.
duration: ""
verification_result: mixed
completed_at: 2026-04-03T17:35:48.327Z
blocker_discovered: false
---

# T01: Reordered the public docs graph and fixed VitePress footer resolution so Clustered Example no longer self-links and proof pages stay out of the footer chain

**Reordered the public docs graph and fixed VitePress footer resolution so Clustered Example no longer self-links and proof pages stay out of the footer chain**

## What Happened

Updated the live VitePress docs graph so the primary onboarding path ends with `Getting Started` and `Clustered Example`, moved `Distributed Proof` and `Production Backend Proof` into a final `Proof Surfaces` sidebar group, and marked those proof links with a sidebar-level footer opt-out. Reworked `usePrevNext.ts` to resolve the current page by exact normalized path, added a no-fallback guard when the current page is not in the footer candidate list, and filtered `includeInFooter: false` items out of the footer chain so proof pages stay public without becoming neighboring prev/next targets. Added `prev: false` / `next: false` frontmatter to both proof pages and created a fail-closed Node contract test that checks sidebar order, proof-link placement/counting, exact-match footer behavior, and proof-page opt-out regressions. Verified the source contract, a real VitePress build, browser-rendered footer behavior on Clustered Example and both proof pages, and the retained M047/reference-backend rails that already exist in this slice.

## Verification

Passed `node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`, `npm --prefix website run build`, `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`, `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`, and `bash reference-backend/scripts/verify-production-proof-surface.sh`. Also previewed the built docs locally and confirmed `Clustered Example` renders Previous -> `/docs/getting-started/` and Next -> `/docs/language-basics/`, while both proof pages render zero `/docs/` footer links. As expected for an intermediate slice task, the future-task surfaces `cargo test -p meshc --test e2e_m050_s01 -- --nocapture` and `bash scripts/verify-m050-s01.sh` still fail because they do not exist until T03.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs` | 0 | ✅ pass | 1120ms |
| 2 | `npm --prefix website run build` | 0 | ✅ pass | 71300ms |
| 3 | `cargo test -p meshc --test e2e_m047_s04 -- --nocapture` | 0 | ✅ pass | 3260ms |
| 4 | `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture` | 0 | ✅ pass | 5000ms |
| 5 | `bash reference-backend/scripts/verify-production-proof-surface.sh` | 0 | ✅ pass | 970ms |
| 6 | `cargo test -p meshc --test e2e_m050_s01 -- --nocapture` | 101 | ❌ fail | 980ms |
| 7 | `bash scripts/verify-m050-s01.sh` | 127 | ❌ fail | 10ms |


## Deviations

Added a sidebar-level `includeInFooter: false` opt-out and filtered it in `usePrevNext.ts`. The written task plan only mentioned exact matching plus proof-page frontmatter, but frontmatter alone does not stop neighboring docs pages from linking into proof pages.

## Known Issues

`cargo test -p meshc --test e2e_m050_s01 -- --nocapture` fails because T03 has not created that test target yet. `bash scripts/verify-m050-s01.sh` fails because T03 has not created that verifier yet. Local VitePress preview still logs `Hydration completed but contains mismatches.`, but built HTML and hydrated footer state matched during this task.

## Files Created/Modified

- `website/docs/.vitepress/config.mts`
- `website/docs/.vitepress/theme/composables/usePrevNext.ts`
- `website/docs/docs/production-backend-proof/index.md`
- `website/docs/docs/distributed-proof/index.md`
- `scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`
- `.gsd/KNOWLEDGE.md`
- `.gsd/DECISIONS.md`


## Deviations
Added a sidebar-level `includeInFooter: false` opt-out and filtered it in `usePrevNext.ts`. The written task plan only mentioned exact matching plus proof-page frontmatter, but frontmatter alone does not stop neighboring docs pages from linking into proof pages.

## Known Issues
`cargo test -p meshc --test e2e_m050_s01 -- --nocapture` fails because T03 has not created that test target yet. `bash scripts/verify-m050-s01.sh` fails because T03 has not created that verifier yet. Local VitePress preview still logs `Hydration completed but contains mismatches.`, but built HTML and hydrated footer state matched during this task.
