---
estimated_steps: 14
estimated_files: 5
skills_used:
  - vitepress
  - test
---

# T01: Reset VitePress onboarding graph and footer matching

**Slice:** S01 — Onboarding Graph & Retained Rail Reset
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

## Inputs

- `website/docs/.vitepress/config.mts`
- `website/docs/.vitepress/theme/composables/usePrevNext.ts`
- `website/docs/docs/production-backend-proof/index.md`
- `website/docs/docs/distributed-proof/index.md`

## Expected Output

- `website/docs/.vitepress/config.mts`
- `website/docs/.vitepress/theme/composables/usePrevNext.ts`
- `website/docs/docs/production-backend-proof/index.md`
- `website/docs/docs/distributed-proof/index.md`
- `scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`

## Verification

- `node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`

## Observability Impact

- Signals added/changed: the new Node graph contract names sidebar-order drift, proof-page footer drift, and `Clustered Example` self-loop regressions before a full site build.
- How a future agent inspects this: run `node --test scripts/tests/verify-m050-s01-onboarding-graph.test.mjs` and inspect the sidebar config plus proof-page frontmatter.
- Failure state exposed: wrong proof-group position, exact-match regression in footer resolution, or missing `prev: false` / `next: false` on proof pages.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `website/docs/.vitepress/config.mts` sidebar graph | Fail the contract test and name the unexpected proof-page position instead of silently accepting any sidebar order. | N/A — source read only. | Reject missing or duplicated proof-page entries as graph drift. |
| `website/docs/.vitepress/theme/composables/usePrevNext.ts` footer matcher | Fail closed if `Clustered Example` still resolves to itself or to a proof page. | N/A — pure source logic. | Treat ambiguous path matches as regression rather than falling back to prefix activation. |
| proof-page frontmatter in `website/docs/docs/{production-backend-proof,distributed-proof}/index.md` | Fail when `prev: false` / `next: false` are missing. | N/A — source read only. | Reject malformed frontmatter that would re-enable proof-page footer chaining. |

## Load Profile

- **Shared resources**: `website/docs/.vitepress/config.mts`, the footer path resolver, and the serialized docs graph emitted into built HTML.
- **Per-operation cost**: one source-level Node contract plus one footer path resolution per affected page; the expensive step is the later site build, not this task’s logic.
- **10x breakpoint**: graph ambiguity fails before throughput does — duplicated sidebar entries or prefix-based matches create self-links and early proof-page routing long before build cost becomes the bottleneck.

## Negative Tests

- **Malformed inputs**: duplicate proof-page sidebar entries, typoed proof-group links, or missing proof-page frontmatter flags.
- **Error paths**: `Clustered Example` still points to itself, `Getting Started` loses `Clustered Example` as the next step, or proof pages regain a footer.
- **Boundary conditions**: `/docs/getting-started/` versus `/docs/getting-started/clustered-example/` exact-match collision, proof pages still reachable by route, and each proof page appearing exactly once in the sidebar graph.
