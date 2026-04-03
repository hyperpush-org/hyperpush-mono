# M050: Public Docs Truth Reset — Context Draft

**Gathered:** 2026-04-03
**Status:** Ready for planning

## Project Description

M050 is the public docs truth reset. It rewrites the public Mesh docs so builders evaluating or actively trying Mesh get one scaffold/examples-first story instead of a proof maze. The public docs should lead with `meshc init --clustered`, `examples/todo-postgres`, and `examples/todo-sqlite`; keep low-level Distributed Actors public; and push verifier maps, milestone rails, and repo-owned proof bundles out of the primary public experience without weakening the underlying proof rigor. This is a docs-surface cleanup and sample-truth pass, not a reduction in internal proof rigor.

## Why This Milestone

M049 already reset the scaffold/example contract, but the public docs still lag behind it. Right now the repo's truthful first-contact surfaces and the docs' first-contact surfaces are not the same thing, so new evaluators and real builders still get routed through proof-map pages, verifier rails, and older proof-app framing before they understand what Mesh actually wants them to do. M050 exists to make the docs match the new public starter truth now, before later site and packages work tries to align around drifting docs.

## User-Visible Outcome

### When this milestone is complete, the user can:

- follow the public docs from install and hello-world into the clustered or Todo starter path without getting sent through internal proof-map material as the main route.
- tell whether they are learning low-level Distributed Actors primitives or the higher-level clustered-app/runtime-owned path, and choose the right follow-on surface.
- reach deeper proof material intentionally when they want it, instead of having that deeper material dominate first-contact onboarding.

### Entry point / environment

- Entry point: `website/docs` public docs pages — especially Getting Started, Clustered Example, Tooling, Distributed Actors, Distributed Proof, and Production Backend Proof — plus linked `examples/*/README.md` and `reference-backend/README.md`
- Environment: browser + local dev + repo docs build + CLI walkthroughs
- Live dependencies involved: local `meshc` / `meshpkg`, docs site build, optional local Postgres for serious-starter sample verification, no required third-party hosted dependency for the core docs-truth path

## Completion Class

- Contract complete means: every touched public docs path has truthful commands and code samples, proof pages are clearly public-secondary instead of coequal first-contact surfaces, and both the assembled docs rail plus page/sample-local checks pass.
- Integration complete means: Getting Started, Clustered Example, Tooling, Distributed Actors, Distributed Proof, Production Backend Proof, generated example READMEs, and deeper `reference-backend` links all tell the same public story with no return to proof-app-shaped onboarding.
- Operational complete means: the docs build cleanly, public navigation and cross-links stop routing first-contact readers into proof-maze material, and docs truth can be rerun mechanically from a clean tree.

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- a builder can install Mesh, follow the public docs into hello-world and then into the clustered or database-specific starter paths, run the documented commands, and keep moving without decoding milestone rails.
- a builder can intentionally branch from the public path into low-level Distributed Actors or deeper backend proof material and understand why that branch exists without mistaking it for the primary onboarding route.
- editorial cleanup alone is not enough: every public command and code sample touched by M050 must be exercised against the real repo, tooling, and docs build through the assembled plus page-local docs-truth rails.

## Risks and Unknowns

- Demoting proof pages might overcorrect and hide useful deeper runbooks builders still need — that would replace the current proof maze with docs that feel shallow or evasive.
- Page-by-page command and sample verification could sprawl or turn flaky if the docs-verifier shape is not constrained early — the milestone needs both layers without inventing a second mega-proof system.
- The split between Distributed Actors primitives and clustered-app/runtime-owned guidance could stay blurry if it is handled only by copy edits — the page roles and link flow need to make the distinction obvious.
- `reference-backend` is still a subtle surface — it should stay public as a deeper backend proof surface, but it must not re-emerge as a coequal first-contact path.
- Terminology retirement can break contributor searchability if older proof-oriented names lose all breadcrumbs at once — public copy should clean up aggressively without making deeper proof material undiscoverable.

## Existing Codebase / Prior Art

- `website/docs/docs/getting-started/index.md` — public first-contact docs that still branch readers toward proof material when they ask whether Mesh is real.
- `website/docs/docs/getting-started/clustered-example/index.md` — already carries the scaffold/examples split, but still feeds readers into proof-map rails.
- `website/docs/docs/tooling/index.md` — central public tool page; mixes user-facing tool docs with assembled verifier/runbook material and already exposes `scripts/verify-m049-s05.sh`.
- `website/docs/docs/distributed/index.md` — low-level `Node.*` / `Global.*` primitives page that currently blends primitive teaching with clustered/operator proof framing.
- `website/docs/docs/distributed-proof/index.md` — public proof map full of repo rails, retained compatibility aliases, and historical fixture-backed commands that should be public-secondary at most.
- `website/docs/docs/production-backend-proof/index.md` — current public entrypoint for the deeper backend proof surface; likely stays public, but secondary.
- `examples/todo-postgres/README.md` and `examples/todo-sqlite/README.md` — M049-generated evaluator-facing follow-on surfaces the public docs should point at first.
- `reference-backend/README.md` and `reference-backend/scripts/verify-production-proof-surface.sh` — deeper backend proof surface plus an existing page-scoped docs-truth verifier model.
- `scripts/verify-m049-s05.sh` — the existing assembled scaffold/example truth rail that can anchor the top-layer docs verifier instead of inventing a second mega-proof.

> See `.gsd/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R117 — primary docs-surface requirement: public docs become evaluator-facing, sample-verified, and stop exposing internal proof-maze material as the main experience.
- R118 — primary guidance requirement: clustered docs need one primary evaluator path, and low-level Distributed Actors primitives need a clearly separate role.
- R116 — supporting boundary from M049: public docs should point at generated `examples/` instead of proof-app-shaped teaching surfaces.
- R120 — downstream alignment requirement: M050 establishes the docs half of the later landing/site/packages coherence story.
- R122 — constraint: docs must preserve the honest split where SQLite stays explicitly local and Postgres remains the serious shared/deployable starter.
- R127 — supporting boundary: `tiny-cluster`, `cluster-proof`, and `reference-backend` must not reappear as coequal first-contact onboarding surfaces.

## Scope

### In Scope

- restructure public docs information architecture and cross-links around scaffold/examples-first evaluator paths.
- demote Distributed Proof and Production Backend Proof to clearly secondary public surfaces rather than main onboarding stops.
- separate low-level distributed primitives from clustered-app/runtime-owned guidance.
- verify public docs commands and code samples command-by-command and sample-by-sample, fixing drift instead of handwaving it.
- add or align a two-layer docs-truth system: one assembled docs verifier plus page/sample-local checks.
- rewrite getting-started, tooling, and distributed pages around the M049 example surfaces and explicit starter-mode wording.

### Out of Scope / Non-Goals

- landing page, packages, or broader site-positioning rewrite — that belongs to M052.
- retiring `reference-backend` in favor of `mesher`, or modernizing `mesher` itself — that belongs to M051.
- deeper clustered deploy, Fly, or load-balancing proof work — that belongs to later milestones.
- deleting retained proof rails or reducing internal proof rigor.
- turning proof pages into hidden or nonexistent material if they still serve as deeper public reference.

## Technical Constraints

- Keep proof pages public, but secondary; they should not stay coequal with the first-contact scaffold/examples path.
- Optimize for builders too — the docs cannot collapse into marketing copy that explains ideas without runnable paths.
- Preserve the M049 starter truth exactly: `meshc init --clustered` is the minimal clustered scaffold, `meshc init --template todo-api --db sqlite` is the honest local starter, and `meshc init --template todo-api --db postgres` is the serious shared/deployable starter.
- Do not reintroduce generic `meshc init --template todo-api` wording where explicit `--db` flags are required to keep the public contract honest.
- Keep low-level `Node.*` / `Global.*` primitives public, but give them a distinct role from the runtime-owned clustered-app path.
- Verification must be both layers: one assembled docs-truth rail plus page/sample-local checks for isolation.
- The docs build and public link graph must stay mechanically verifiable from a clean repo tree.

## Integration Points

- `website/docs/docs/getting-started/index.md` — public first-contact path.
- `website/docs/docs/getting-started/clustered-example/index.md` — scaffold-first clustered path and example branching.
- `website/docs/docs/tooling/index.md` — public CLI/tooling surface and top-layer verifier discoverability.
- `website/docs/docs/distributed/index.md` — low-level primitives page that needs a cleaner boundary from clustered guidance.
- `website/docs/docs/distributed-proof/index.md` and `website/docs/docs/production-backend-proof/index.md` — deeper proof surfaces that remain public-secondary.
- `examples/todo-postgres/README.md` and `examples/todo-sqlite/README.md` — first follow-on surfaces the public docs should link into.
- `reference-backend/README.md` — deeper backend proof surface, public but not coequal first-contact.
- `scripts/verify-m049-s05.sh` and page-local verifiers such as `reference-backend/scripts/verify-production-proof-surface.sh` — docs-truth enforcement seams.
- `npm --prefix website run build` — public docs build gate.

## Open Questions

- How should the public docs nav/sidebar expose proof pages while keeping them public-secondary? — current thinking: reachable from deeper sections and follow-on links, not from the first-step happy path.
- What is the cleanest page-local truth mechanism under the assembled docs rail? — current thinking: page- or sample-specific checks that can fail in isolation while one named assembled command still proves the whole docs story.
- How aggressive should public terminology retirement be for older proof-oriented names? — current thinking: aggressive in first-contact copy, with contributor and search breadcrumbs preserved only where deeper proof pages still need them.
- Should `reference-backend` appear directly in nav or only as a deeper follow-on link from relevant pages? — current thinking: keep it public as a deeper backend proof surface, but not as a coequal onboarding waypoint.
