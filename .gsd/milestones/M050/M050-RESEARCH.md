# M050 Research — Public Docs Truth Reset

## Skills Discovered

- `vitepress` — already installed and directly relevant to `website/docs/.vitepress/config.mts`, sidebar/nav wiring, and file-based routing. No extra install needed.
- `bash-scripting` — already installed and directly relevant to the repo’s fail-closed docs verifier pattern. No extra install needed.

No additional skill discovery or installs were necessary; the core technologies for this milestone are already covered.

## Baseline Checks

These baseline checks are green right now, so M050 is starting from a mechanically healthy state rather than a red tree:

| Command | Result | Notes |
|---|---|---|
| `npm --prefix website run build` | ✅ pass | VitePress build completed in ~65s; only chunk-size warnings surfaced. |
| `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` | ✅ pass | Current scaffold/examples-first onboarding contract is enforced. |
| `node --test scripts/tests/verify-m049-s05-contract.test.mjs` | ✅ pass | Current assembled M049 S05 verifier contract and docs discoverability are enforced. |
| `bash reference-backend/scripts/verify-production-proof-surface.sh` | ✅ pass | Production backend proof page + runbook parity is mechanically verified today. |

## What Exists Now

### Site tech and navigation mechanics

- The docs site is **VitePress 1.6.4** (`website/package.json`), not Docusaurus.
- Sidebar structure is centralized in `website/docs/.vitepress/config.mts`.
- Proof pages are currently first-class sidebar entries:
  - `Production Backend Proof` under **Getting Started**
  - `Distributed Proof` under **Distribution**
- The custom prev/next footer logic in `website/docs/.vitepress/theme/composables/usePrevNext.ts` flattens the sidebar order and uses that as the navigation sequence.
- No page currently sets `prev: false` or `next: false`.

**Implication:** sidebar order is not cosmetic. It is the real onboarding graph. If proof pages stay in the current sidebar positions, they remain part of the default happy path even if the prose gets cleaned up.

### Public surfaces already closest to the target story

- `website/docs/docs/getting-started/clustered-example/index.md` is already mostly scaffold/examples-first.
- `examples/todo-postgres/README.md` and `examples/todo-sqlite/README.md` already carry the explicit Postgres-vs-SQLite truth from M049.
- `website/docs/docs/production-backend-proof/index.md` + `reference-backend/README.md` already form a reasonable **secondary** proof/runbook pair.

### Biggest current drift

- `website/docs/docs/getting-started/index.md` opens by redirecting evaluators to **Production Backend Proof** instead of branching them into the scaffold/example path after hello-world.
- `website/docs/docs/tooling/index.md` puts release/proof runbooks before formatter/REPL/package-manager material.
- `website/docs/docs/distributed/index.md` has a giant proof-rail intro even though the body is really a low-level primitive guide.
- `website/docs/docs/distributed-proof/index.md` is still a public proof map full of milestone rails, fixture paths, and docs-build commands. That can work as a secondary page, not as a coequal onboarding stop.

### Cross-link sprawl

A quick search across README + docs + VitePress config found:

- **11** files referencing `reference-backend/README.md`
- **8** files referencing `/docs/production-backend-proof/`
- **5** files referencing `/docs/distributed-proof/`
- **5** public-surface files referencing `verify-m047*` rails directly

The problem is not missing proof links. The problem is **proof over-prominence on first-contact surfaces**.

## Verification / Truth Seams to Reuse

The repo already has most of the raw pieces for a two-layer docs-truth system.

| Need | Existing seam | What it means for M050 |
|---|---|---|
| Top-layer scaffold/examples truth | `bash scripts/verify-m049-s05.sh` + `node --test scripts/tests/verify-m049-s05-contract.test.mjs` | Reuse this wrapper pattern instead of rebuilding runtime/sample proof from scratch. |
| Example README truth | `node scripts/tests/verify-m049-s03-materialize-examples.mjs --check` + `cargo test -p meshc --test e2e_m049_s03 -- --nocapture` | Example commands are already mechanically proven; page-local README truth should delegate here rather than invent a new parser-heavy proof. |
| Clustered onboarding copy/link contract | `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` | Fast source-level contract already exists, though M050 will likely need to retarget some expectations. |
| Backend proof page/runbook parity | `bash reference-backend/scripts/verify-production-proof-surface.sh` | This is the best current model for a secondary proof-surface verifier. |
| Built-site gate | `npm --prefix website run build` | Expensive (~65s). Keep it as the assembled gate, not the per-page mechanism. |
| Built/public-surface verification precedent | `scripts/lib/m034_public_surface_contract.py` | There is already a pattern for source-doc + built-dist verification if M050 needs HTML-level nav assertions. |

## Critical Constraints and Surprises

### 1. Active M047 rails are still mechanical blockers

This is the most important research finding.

M049’s top-level assembled verifier still replays M047 docs-era rails:

- `scripts/verify-m049-s05.sh` replays `bash scripts/verify-m047-s05.sh`
- `scripts/verify-m047-s05.sh` replays `bash scripts/verify-m047-s04.sh`

Those retained rails are not light wrappers. Their Rust and shell contracts still assert exact strings in current public docs surfaces:

- `compiler/meshc/tests/e2e_m047_s04.rs`
- `compiler/meshc/tests/e2e_m047_s05.rs`
- `scripts/verify-m047-s04.sh`
- `scripts/verify-m047-s05.sh`

They currently expect things like:

- `bash scripts/verify-m047-s04.sh` references in public docs
- legacy migration markers like `execute_declared_work(...)` / `Work.execute_declared_work`
- explicit proof-rail references across `tooling`, `clustered-example`, `distributed`, and `distributed-proof`

**Implication:** M050 cannot just rewrite prose. It has to retarget or relax these active retained docs-contract rails first or alongside the copy changes, otherwise the current assembled M049 proof chain will fail.

### 2. Example README edits are generator-owned edits

`examples/todo-postgres/README.md` and `examples/todo-sqlite/README.md` are generated surfaces backed by `compiler/mesh-pkg/src/scaffold.rs` and the M049 materializer flow.

**Implication:** if M050 changes those README surfaces, it is not “just markdown.” It becomes scaffold/generator work with parity-test blast radius.

If those readmes do not need wording changes, the cheapest path is to **leave them alone** and repoint the website docs at them more cleanly.

### 3. Proof-page demotion is partly nav/footer work, not just copy work

Because `usePrevNext.ts` derives prev/next from the flattened sidebar and no page disables it, proof pages remain first-contact by structure as long as they stay where they are in `config.mts`.

A VitePress page does **not** need a sidebar entry to stay public. File-based routing already keeps it reachable by URL.

**Implication:** one clean demotion mechanism is to remove proof pages from the primary sidebar path and link them intentionally from deeper sections. If they remain in the sidebar, the planner should still treat prev/next/frontmatter behavior as part of the acceptance contract.

### 4. The docs-truth system is half-built already

The repo already has:

- top-layer sample truth (`verify-m049-s05.sh`)
- example parity/build truth (`verify-m049-s03` + `e2e_m049_s03`)
- a mature backend proof-surface verifier (`reference-backend/scripts/verify-production-proof-surface.sh`)
- historical distributed proof-surface verifier patterns (`scripts/verify-m039-s04-proof-surface.sh`, `scripts/verify-m042-s04-proof-surface.sh`, `scripts/verify-m043-s04-proof-surface.sh`)

What is missing is **an M050-shaped contract** for:

- first-contact docs sequencing
- current distributed proof page semantics as a secondary surface
- built-site confirmation that the new graph renders the intended navigation

### 5. `reference-backend` is still intentionally alive until M051

M050 is not the retirement milestone for `reference-backend/`.

**Implication:** M050 should demote it to a deeper public runbook, not try to replace it with `mesher/` or collapse it into the primary onboarding path. The docs work should avoid creating even more dependence on `reference-backend`, but it also should not try to delete or hide it.

## What Should Be Proven First?

1. **Prove the new docs contract shape, not the prose.**
   - First retire/update the mechanical blockers in active M047 docs rails and any sidebar assumptions in `reference-backend/scripts/verify-production-proof-surface.sh`.
   - Otherwise every content edit will fight retained exact-string verifiers.

2. **Prove the happy-path onboarding graph.**
   - Sidebar + prev/next + first-contact cross-links must route readers to `meshc init --clustered`, `examples/todo-postgres`, and `examples/todo-sqlite` before proof pages.

3. **Prove secondary proof surfaces separately.**
   - `distributed-proof` and `production-backend-proof` should each have a small fail-closed surface verifier of their own.

4. **Prove the built site once at the top.**
   - Keep `npm --prefix website run build` as the assembled gate instead of repeating it in every page-local verifier.

## Suggested Slice Boundaries

### Slice 1 — Docs contract and navigation reset

**Goal:** make the onboarding graph match M049 truth before rewriting every page.

Likely surfaces:

- `website/docs/.vitepress/config.mts`
- first-contact source-contract tests/verifiers that currently hard-code proof-rail presence:
  - `compiler/meshc/tests/e2e_m047_s04.rs`
  - `compiler/meshc/tests/e2e_m047_s05.rs`
  - `scripts/verify-m047-s04.sh`
  - `scripts/verify-m047-s05.sh`
- possibly `scripts/verify-m047-s06.sh` / `compiler/meshc/tests/e2e_m047_s06.rs` if the planner wants that retained rail to stay runnable against the new docs contract
- any new fast source-level M050 docs contract test(s)

Why first:

- These are the mechanical blockers that will make ordinary doc rewrites fail.
- This slice also answers the open nav/sidebar question concretely.

### Slice 2 — First-contact page rewrite

**Goal:** rewrite the evaluator-facing docs path without proof-maze detours.

Likely surfaces:

- `website/docs/docs/getting-started/index.md`
- `website/docs/docs/getting-started/clustered-example/index.md`
- `website/docs/docs/tooling/index.md`

Optional only if the planner decides GitHub-first contact must be aligned now:

- `README.md`

Expected outcome:

- hello-world branches clearly into clustered, SQLite-local, or Postgres shared/deployable paths
- tooling keeps verifier/runbook material deeper than the core tool docs
- first-contact pages no longer teach proof pages as the next obvious stop

### Slice 3 — Secondary proof surface cleanup + page-local verifiers

**Goal:** keep proof material public but explicitly secondary, and make it mechanically truthful.

Likely surfaces:

- `website/docs/docs/distributed/index.md`
- `website/docs/docs/distributed-proof/index.md`
- `website/docs/docs/production-backend-proof/index.md`
- `reference-backend/README.md`
- `reference-backend/scripts/verify-production-proof-surface.sh`
- new/updated docs-verifier scripts/tests for distributed proof / first-contact docs

Only touch these if wording truly needs to change:

- `examples/todo-postgres/README.md`
- `examples/todo-sqlite/README.md`
- `compiler/mesh-pkg/src/scaffold.rs`

Otherwise they are already the best public truth anchors.

## Requirement Analysis

### Table stakes from current active requirements

- **R117** — first-contact docs must stop making verifier maps, milestone rails, and repo proof bundles the main user experience.
- **R118** — low-level distributed primitives vs runtime-owned clustered apps must be obviously different learning paths.
- **R116** — public docs must keep routing into generated `examples/` instead of proof-app-shaped teaching surfaces.
- **R122** — every touched page must preserve the honest split: SQLite is local/single-node, Postgres is the serious shared/deployable starter.

### Supporting but not core to M050

- **R120** matters only on the docs half for this milestone. Landing/packages work is later.

### Important anti-feature boundary

- **R127** is the right anti-feature for the wave, but M050 only advances the **demotion** part of it. Full replacement/retirement of `reference-backend` belongs to M051.

## Candidate Requirements (Advisory, Not Yet Binding)

1. **Public docs navigation must not place proof pages in the default prev/next happy path for Getting Started and Clustered Example.**
   - R117 implies this, but there is no explicit current requirement guarding sidebar/footer sequencing.

2. **M050 docs truth should verify both source docs and built-site navigation while delegating runtime/sample truth to existing M049/M028 rails.**
   - This matters because VitePress build alone will not catch drift in externally linked GitHub README runbooks.

3. **The distributed proof surface needs a modern page-local verifier on its current secondary/public contract.**
   - Historical M039–M043 proof-surface scripts and the active M047 exact-string rails are not the same thing as an M050 secondary-surface contract.

## Recommended Implementation Posture

- Reuse existing proofs; do not invent a new mega-verifier.
- Keep page-local verifiers fast and source-based.
- Run the expensive VitePress build once in the assembled M050 rail.
- Prefer link-graph and role-label changes over large body rewrites where the underlying technical content is already good.
- Avoid unnecessary generator edits; the example runbooks are already strong truth anchors.

## Direct Answers to the Milestone Open Questions

- **How should nav/sidebar expose proof pages while keeping them public-secondary?**
  - Keep them public by route and deliberate links, not by leaving them as coequal sidebar steps in the primary onboarding sequence. If they must stay in the sidebar, treat order and prev/next behavior as part of the contract.

- **What is the cleanest page-local truth mechanism under the assembled docs rail?**
  - Fast exact-string/source contract tests or small fail-closed shell verifiers per surface, with example/runtime truth delegated to existing `verify-m049-s03` / `e2e_m049_s03` and backend proof truth delegated to `reference-backend/scripts/verify-production-proof-surface.sh`.

- **How aggressive should terminology retirement be?**
  - Aggressive on first-contact pages. Preserve breadcrumbs only on secondary proof pages and retained historical verifier surfaces.

- **Should `reference-backend` appear directly in nav or only as a deeper follow-on link?**
  - Research strongly favors deeper follow-on link, not a Getting Started coequal sidebar stop.
