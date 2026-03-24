# S06: Honest Production Proof and Documentation — Research

## Summary

S06 directly owns **R008** and **R009**. It also supports **R005** and **R006** by turning the already-built backend/tooling/deploy proof into a public proof surface instead of leaving it buried in package-local docs and test files.

The main finding is that S06 cannot be planned as "just docs polish." The current public surface is split across three different truth levels:

1. **Public docs and README are still mostly toy-first.**
   - `README.md` still leads with hello-world / toy web examples and broad reliability/deployment claims.
   - `website/docs/docs/getting-started/index.md`, `website/docs/docs/web/index.md`, `website/docs/docs/databases/index.md`, and `website/docs/docs/concurrency/index.md` are generic feature guides, not a production-proof narrative.
   - Outside of tooling/testing mentions, the public docs do not lead readers to `reference-backend/` as the canonical backend proof.

2. **`reference-backend/README.md` is the only real operator/developer proof surface, but it is package-local and incomplete.**
   - It already documents S01-S04 well: startup contract, migrations, build/run/test, staged deploy bundle, and smoke verification.
   - It does **not** yet contain the planned S05 `Supervision and recovery` section.

3. **The proof path is currently regressed, so several docs already overstate reality.**
   - `cargo run -p meshc -- build reference-backend` currently fails with a parse error in `reference-backend/jobs/worker.mpl` around line 327.
   - `cargo run -p meshc -- test reference-backend` currently fails on the same parse error.
   - `cargo run -p meshc -- fmt --check reference-backend` currently fails because `reference-backend/jobs/worker.mpl` would be reformatted.
   - `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture` currently fails because the package no longer builds.
   - `reference-backend/README.md`, `website/docs/docs/tooling/index.md`, and `website/docs/docs/testing/index.md` currently advertise command paths that are not green in this worktree.

That means the first S06 planner decision is architectural, not editorial:

**Do not promote the proof path further until the proof path is truthful again.**

This follows the loaded `debug-like-expert` rule: **VERIFY, DON’T ASSUME.** It also follows the loaded `test` rule: keep extending the existing proof surfaces instead of inventing new ones.

## Recommendation

Plan S06 in two phases.

### Phase 0 — restore honesty baseline before broader doc promotion

Treat these as hard prerequisites for any public-facing production-proof promotion:

- `cargo run -p meshc -- build reference-backend`
- `cargo run -p meshc -- fmt --check reference-backend`
- `cargo run -p meshc -- test reference-backend`
- `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture`

Today, all four are red. Until they are green again, S06 should not present `reference-backend/` as a healthy canonical proof path.

This is effectively a carry-forward dependency from S05, not because S06 should own worker/runtime fixes, but because **honest docs cannot outrun a broken proof target**.

### Phase 1 — build one public proof narrative around `reference-backend/`

Once the baseline is green again, the cleanest S06 shape is:

1. **Keep `reference-backend/README.md` as the package-local runbook**
   - full command surface
   - exact repo-level proofs
   - deploy scripts
   - eventual supervision/recovery section once S05 proof is real

2. **Add one website page that explains the production proof story end to end**
   - audience: outside backend evaluator or repo reader
   - explain what `reference-backend/` proves
   - summarize the proof classes: tooling, runtime, deploy, recovery
   - link to the exact package README and exact repo tests/scripts instead of duplicating every command block

3. **Update `README.md` to point to that proof surface near the top**
   - one explicit “Backend proof” section
   - stop making deployment/reliability claims without a direct link to the proof path
   - make it obvious that the real proof target is `reference-backend/`, not scattered toy snippets

4. **Add a small doc-truth sweep while touching these surfaces**
   - fix the stale install URL in `website/docs/docs/getting-started/index.md` (`mesh-lang.org` vs `meshlang.dev`)
   - remove the `(placeholder link)` wording in `README.md`
   - decide whether the `v12.0` README version text should be synced with the website’s `meshVersion: '14.0'` or removed from the public docs if it is not maintained carefully

## What exists now

### `reference-backend/README.md`

This is already the strongest documentation surface in the repo.

What it already covers well:
- startup contract (`DATABASE_URL`, `PORT`, `JOB_POLL_MS`)
- local prerequisites
- S04 artifact-first deploy path
- staged bundle layout
- runtime-host SQL apply path
- staged smoke script
- canonical build/test/fmt/LSP/migrate/run commands
- compiler-facing proof commands for build, runtime start, deploy artifact smoke, and Postgres smoke

What is still missing:
- the S05 `Supervision and recovery` section planned in `.gsd/milestones/M028/slices/S05/S05-PLAN.md`
- any warning that some currently listed commands are red in the worktree right now

Planner implication:
- this file should remain the deepest repo-local runbook
- S06 should extend it, not replace it
- do not fork its commands into multiple independent doc surfaces unless there is a very strong reason

### `README.md`

This is still marketing-first and toy-first.

Important current traits:
- claims supervision/fault tolerance broadly (`README.md:33`)
- claims deployment is easy via native binaries (`README.md:38`)
- does **not** mention `reference-backend/` anywhere
- has a docs sentence that still says `meshlang.dev` is a `(placeholder link)` (`README.md:131`)

Planner implication:
- this is the first page external evaluators will read
- S06 should add one clear, early pointer to the real backend proof path
- it should stop relying on broad claims with no immediate evidence trail

### Website docs

#### `website/docs/docs/getting-started/index.md`
- still uses `curl -sSf https://mesh-lang.org/install.sh | sh` (`:25`), which does not match the actual install surfaces under `meshlang.dev`
- remains purely beginner/tutorial focused

#### `website/docs/docs/web/index.md`
- has only toy HTTP examples
- does not point to `reference-backend/` as the production-style backend proof

#### `website/docs/docs/databases/index.md`
- has useful API-level Pool/Postgres guidance, but not a real backend path
- no link to the one package that proves HTTP + DB + migrations + jobs together

#### `website/docs/docs/concurrency/index.md`
- presents supervision as a feature guide with toy examples
- does not distinguish language surface docs from the still-hardening backend proof surface

#### `website/docs/docs/tooling/index.md` and `website/docs/docs/testing/index.md`
- already mention `reference-backend` commands
- currently overstate reality because the referenced command paths are red in this worktree

Planner implication:
- do **not** smear the production-proof story across every doc page
- add one dedicated page, then use a few cross-links/callouts from the generic docs
- any touched command examples need live reruns, not copy edits by inspection

### `website/docs/.vitepress/config.mts`

The sidebar is fully manual.

Planner implication:
- any new docs page must be added explicitly to the sidebar
- there is no autogenerated section that will pick up a new file automatically

### `.gsd/milestones/M028/slices/S05/S05-SUMMARY.md`

This is still a doctor-created placeholder, and the S05 task summaries explicitly say the slice is not actually complete.

Planner implication:
- do not cite the placeholder as authoritative proof
- if S06 wants to summarize supervision/recovery proof publicly, it should wait until the underlying S05 commands pass and the real S05 summary is rewritten

## Current verified state

### Commands I ran

```bash
cargo run -p meshc -- build reference-backend
cargo run -p meshc -- fmt --check reference-backend
cargo run -p meshc -- test reference-backend
cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture
cargo test -p meshc --test e2e_supervisors -- --nocapture
npm --prefix website run build
rg -n "Supervision and recovery|restart_count|last_exit_reason|recovered_jobs|process restart" reference-backend/README.md
npx skills find "Rust"
npx skills find "PostgreSQL"
npx skills find "VitePress"
```

### What those commands proved

- `meshc build reference-backend` is currently broken by a parse error in `reference-backend/jobs/worker.mpl` around line 327.
- `meshc fmt --check reference-backend` currently fails because `reference-backend/jobs/worker.mpl` would be reformatted.
- `meshc test reference-backend` fails on the same parse error.
- the build-only `e2e_reference_backend_builds` regression also fails for the same reason.
- `e2e_supervisors` still passes, but the file remains a source-level supervisor smoke surface, not the actual backend proof surface.
- `reference-backend/README.md` still has no supervision/recovery section.
- `npm --prefix website run build` currently fails locally because `vitepress` is not installed in the worktree environment (`sh: vitepress: command not found`).

## Natural seams for planning

### Seam 1: restore proof-surface truth before doc promotion

Files / surfaces:
- `reference-backend/jobs/worker.mpl`
- `reference-backend/api/health.mpl`
- `reference-backend/storage/jobs.mpl`
- `reference-backend/main.mpl`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `reference-backend/README.md`

Why this matters:
- S06 cannot honestly publish a broken proof path
- the current breakage is specific and reproducible, not abstract

Planner note:
- this may need to be treated as a dependency-repair task before the actual documentation tasks begin
- at minimum, gate all S06 doc edits on rerunning the green command set

### Seam 2: add one canonical public “production backend proof” page

Likely files:
- new docs file under `website/docs/docs/...` (best fit is a dedicated page, not another huge section inside `tooling`)
- `website/docs/.vitepress/config.mts`

Why this is the right seam:
- one page can narrate the whole proof path cleanly
- it avoids duplicating long command lists in every guide
- it gives the sidebar a stable entry point for backend evaluators

Recommended content shape:
- what `reference-backend/` is
- what it proves today
- exact proof classes and where each one lives
  - tooling proof
  - runtime correctness proof
  - boring deployment proof
  - supervision/recovery proof (only once real)
- where to go next: package README, e2e harness, deploy scripts

### Seam 3: top-level README promotion

Files:
- `README.md`

Why this matters:
- this is the repo landing page
- the milestone explicitly wants docs/examples to stop implying readiness through toys alone

Recommended shape:
- add a short “Production backend proof” section near the top
- point directly to:
  - `reference-backend/README.md`
  - the website production-proof page
  - `compiler/meshc/tests/e2e_reference_backend.rs` as the authoritative regression surface
- trim or re-anchor broad claims so they are adjacent to proof links

### Seam 4: cross-link generic docs instead of rewriting them wholesale

Files:
- `website/docs/docs/getting-started/index.md`
- `website/docs/docs/web/index.md`
- `website/docs/docs/databases/index.md`
- `website/docs/docs/concurrency/index.md`
- possibly `website/docs/docs/tooling/index.md` and `website/docs/docs/testing/index.md`

Why this is a separate seam:
- these pages are still useful as feature guides
- the main issue is discoverability of the real proof path, not that every feature guide must become a reference-backend manual

Recommended change style:
- add brief callouts/links to the dedicated proof page and package README
- keep commands here only if they are rerun live
- fix obvious doc-truth drift while touching them (e.g. install domain)

### Seam 5: website build verification setup

Files / surfaces:
- `website/package.json`
- `website/package-lock.json`
- local node_modules state (not committed)

Why this matters:
- VitePress build is the obvious docs-build gate
- it is not currently runnable in this worktree without installing dependencies

Planner implication:
- if the slice wants a real docs build gate, it likely needs:
  - `npm --prefix website ci`
  - then `npm --prefix website run build`
- because that install can take time, it should be planned as a distinct verification step, preferably with background execution if needed

## What to build or prove first

1. **First restore the reference-backend baseline to green.**
   - Build/test/fmt/e2e-build are red today.
   - Until they are green, broader doc promotion is not honest.

2. **Then write the single public production-proof page and wire the sidebar.**
   - This gives the rest of the doc changes a stable target.

3. **Then update `README.md` to point to that proof surface early.**
   - This is the highest-leverage public-facing doc change.

4. **Then add minimal cross-links from generic docs.**
   - Keep those edits small and focused.

5. **Only after S05 proof is real, extend `reference-backend/README.md` with supervision/recovery and promote that story publicly.**
   - Right now the package README is missing that section, and the underlying proof is still incomplete.

## Verification plan

### Prerequisite truth gates

These should be green before calling the production-proof docs honest:

```bash
cargo run -p meshc -- build reference-backend
cargo run -p meshc -- fmt --check reference-backend
cargo run -p meshc -- test reference-backend
cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture
```

Then re-run the already-established S04/S05-facing proofs as applicable:

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture
```

The S05 commands should only appear in public docs if they are actually passing in this slice’s final state.

### Docs-build gate

After website dependencies are present:

```bash
npm --prefix website ci
npm --prefix website run build
```

### Doc-truth grep sweep

Use a targeted negative sweep so stale phrases fail mechanically.

At minimum, sweep for:

```bash
rg -n "mesh-lang.org/install.sh|placeholder link" README.md website/docs/docs/getting-started/index.md
```

And verify the new proof-surface links exist where expected.

### Package-local supervision docs gate

When S05 is actually real, use the planned grep gate from S05:

```bash
rg -n "Supervision and recovery|restart_count|last_exit_reason|recovered_jobs|process restart" reference-backend/README.md
```

## Risks / gotchas

- **Biggest risk: S06 turns into dishonest documentation of an unfinished S05.**
  - The current worktree already shows how easy that drift is: package/docs surfaces list commands that are red today.

- **Do not duplicate the backend proof app into a separate example package.**
  - The honest proof target is already `reference-backend/`.
  - Duplicating it into `examples/` would create another truth-maintenance problem.

- **Do not smear long command lists across README + website + package README independently.**
  - Keep `reference-backend/README.md` as the deepest runbook.
  - Let README/website summarize and link.

- **Remember that the VitePress sidebar is manual.**
  - A new page without a config update will be effectively hidden.

- **Be explicit about audience when choosing command forms.**
  - repo-clone evaluator docs can use `cargo run -p meshc -- ...`
  - installed-tool docs can use `meshc ...`
  - mixing the two without explanation makes the docs feel sloppier than they need to.

## Skill discovery

Installed skills already relevant here:
- `debug-like-expert` — use its `VERIFY, DON'T ASSUME` rule for all proof-status claims
- `test` — extend existing proof surfaces instead of inventing new ones
- `review` — read full surrounding files before calling drift or overclaiming a surface

Promising missing skills found during research (do not install automatically):
- Rust: `npx skills add apollographql/skills@rust-best-practices`
- PostgreSQL: `npx skills add github/awesome-copilot@postgresql-optimization`
- VitePress: `npx skills add antfu/skills@vitepress`

## Sources

Files inspected:
- `README.md`
- `reference-backend/README.md`
- `reference-backend/main.mpl`
- `reference-backend/runtime/registry.mpl`
- `reference-backend/jobs/worker.mpl`
- `reference-backend/api/health.mpl`
- `reference-backend/storage/jobs.mpl`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `compiler/meshc/tests/e2e_supervisors.rs`
- `website/docs/.vitepress/config.mts`
- `website/docs/docs/getting-started/index.md`
- `website/docs/docs/web/index.md`
- `website/docs/docs/databases/index.md`
- `website/docs/docs/concurrency/index.md`
- `website/docs/docs/tooling/index.md`
- `website/docs/docs/testing/index.md`
- `website/package.json`
- `.gsd/milestones/M028/slices/S05/S05-PLAN.md`
- `.gsd/milestones/M028/slices/S05/S05-RESEARCH.md`
- `.gsd/milestones/M028/slices/S05/S05-SUMMARY.md`
- `.gsd/milestones/M028/slices/S05/tasks/T01-SUMMARY.md`
- `.gsd/milestones/M028/slices/S05/tasks/T02-SUMMARY.md`
- `.gsd/milestones/M028/slices/S05/tasks/T03-SUMMARY.md`
- `.gsd/milestones/M028/slices/S05/tasks/T04-SUMMARY.md`

Key repo decisions / context used:
- D018 / D019 artifact-first deployment pattern
- D020 S05 sequencing around supervision/recovery proof before promotion
- S01-S04 slice summaries from the preloaded milestone context
