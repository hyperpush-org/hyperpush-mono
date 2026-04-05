# S04: Retarget public docs, scaffold, and skills to the examples-first story

**Goal:** Retarget the public docs, generated clustered scaffold guidance, and bundled clustering skill so public readers land on scaffold/examples-first surfaces, while Mesher becomes the maintainer-facing deeper app and the retained backend proof stays behind named maintainer verifiers instead of repo-root `reference-backend/` teaching.
**Demo:** After this: A public reader following README, VitePress docs, scaffold output, or bundled skill guidance lands on scaffold output and `/examples`, while Mesher is described only as the deeper maintained app for repo maintainers.

## Tasks
- [x] **T01: Verified the first-contact docs already ship the examples-first ladder and captured the remaining downstream stale rails for later S04 tasks.** — ### Why
The public first-contact path still teaches `reference-backend/README.md` as the next step after the starter chooser, and `website/docs/docs/tooling/index.md` still publishes stale public `meshc test reference-backend` and `meshc fmt --check reference-backend` commands even though S02 and S03 already moved the truthful deeper/backend/tooling proof elsewhere. This task closes the outward-facing first-contact drift before the deeper proof-page and compatibility rails are rewritten.

### Steps
1. Update `README.md` so the public “Where to go next” ladder stays starter/examples-first, routes deeper backend proof through `/docs/production-backend-proof/` instead of direct `reference-backend/README.md`, and keeps Mesher references in maintainer-only guidance rather than the user-facing starter path.
2. Rewrite `website/docs/docs/getting-started/index.md` and `website/docs/docs/getting-started/clustered-example/index.md` so the three-way starter chooser stays explicit, the follow-on ladder ends at the Production Backend Proof page instead of a direct repo-root runbook handoff, and the public clustered tutorial remains scaffold/examples-first.
3. Rework `website/docs/docs/tooling/index.md` so it keeps the public CLI workflow first, removes stale public `meshc test reference-backend` / `meshc fmt --check reference-backend` examples, and routes deeper backend proof through the proof page rather than the repo-root compatibility copy.
4. Update `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs` so the first-contact docs contract fails closed on the new examples-first / maintainer-only deeper-reference wording and on any reintroduction of direct repo-root backend commands into the public first-contact path.

### Must-Haves
- [ ] `README.md`, Getting Started, Clustered Example, and Tooling no longer send public readers directly to `reference-backend/README.md` as the next step after the starter chooser.
- [ ] The three-way starter chooser and ordered first-contact path remain explicit and still distinguish `meshc init --clustered`, SQLite todo, and Postgres todo.
- [ ] Public Tooling docs no longer publish `meshc test reference-backend` or `meshc fmt --check reference-backend` as day-one commands.
- [ ] `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs` asserts the new wording and ordering instead of the old repo-root backend handoff.

### Done when
The public first-contact docs all point readers through the examples-first ladder and the updated first-contact Node contract passes against the new copy.
  - Estimate: 1h30m
  - Files: README.md, website/docs/docs/getting-started/index.md, website/docs/docs/getting-started/clustered-example/index.md, website/docs/docs/tooling/index.md, scripts/tests/verify-m050-s02-first-contact-contract.test.mjs
  - Verify: `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`
- [x] **T02: Retargeted the public-secondary backend docs and proof verifiers to Mesher plus named retained backend replays.** — ### Why
The public-secondary docs graph still describes `reference-backend/README.md` as the deeper backend runbook almost everywhere, and the compatibility verifier at `reference-backend/scripts/verify-production-proof-surface.sh` still enforces the old story. This task keeps the existing public route structure and proof-page placement, but changes the story behind it so the deeper app is Mesher for maintainers and the retained backend-only proof survives behind named verifier commands instead of a public fixture path.

### Steps
1. Rewrite `website/docs/docs/production-backend-proof/index.md` in place so it stays the compact public-secondary handoff at the same route and footer settings, but its canonical surfaces and named commands now explain the new split: starter/examples first for public readers, `mesher/README.md` plus `bash scripts/verify-m051-s01.sh` for the deeper maintained app, and `bash scripts/verify-m051-s02.sh` for the retained backend-only maintainer proof.
2. Update `website/docs/docs/distributed/index.md`, `website/docs/docs/distributed-proof/index.md`, `website/docs/docs/web/index.md`, `website/docs/docs/databases/index.md`, `website/docs/docs/testing/index.md`, and `website/docs/docs/concurrency/index.md` so their backend handoffs go through Production Backend Proof and the new maintainer-facing deeper-reference story rather than direct repo-root `reference-backend/README.md` teaching.
3. Rewrite `reference-backend/scripts/verify-production-proof-surface.sh` to keep the file path alive through S04 while verifying the new public-secondary contract, named maintainer commands, and proof-page role instead of the old repo-root backend story.
4. Update `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` so it fails closed on stale repo-root backend handoffs, missing Mesher / retained-proof markers, or proof-page role drift.

### Must-Haves
- [ ] `/docs/production-backend-proof/` keeps its public-secondary route, sidebar role, and footer opt-out, but no longer acts like a public repo-root `reference-backend/` runbook.
- [ ] Secondary guides hand readers through Production Backend Proof before any deeper maintainer-only surfaces and stop teaching the repo-root compatibility copy as the next public step.
- [ ] The new proof-page contract names Mesher as the maintained deeper app for repo maintainers and the retained backend-only proof as a named maintainer verifier, without exposing `scripts/fixtures/backend/reference-backend/` publicly.
- [ ] `reference-backend/scripts/verify-production-proof-surface.sh` still exists and fail-closes on proof-page contract drift.

### Failure Modes
| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `reference-backend/scripts/verify-production-proof-surface.sh` | Stop on the first missing marker and name the drifting file or command explicitly | Fail closed; do not silently skip proof-page checks | Treat wrong public commands or missing role markers as contract drift |
| Built docs graph | Keep route/role assertions tied to the existing page paths instead of inventing a new route | Use bounded verifier timeouts and stop on the first failing phase | Treat malformed built HTML or missing proof-page markers as verifier failure, not acceptable docs churn |
| Secondary-surface Node contract | Fail closed on missing or reintroduced links/commands in any guide | N/A for source assertions | Treat order or wording drift as a real contract breakage |

### Load Profile
- **Shared resources**: `website/docs/.vitepress/dist/`, the proof-page verifier artifact logs, and the public docs source tree.
- **Per-operation cost**: one Node source contract and one compatibility verifier replay plus bounded Markdown rewrites.
- **10x breakpoint**: docs build and built-HTML inspection dominate first; the source-only edits themselves are light.

### Negative Tests
- **Malformed inputs**: missing proof-page headings, direct fixture-path leakage, or stale repo-root runbook commands in the public docs.
- **Error paths**: proof-page route/role drift, broken sidebar/footer assumptions, or compatibility verifier checks that no longer match the page contract.
- **Boundary conditions**: Production Backend Proof stays public-secondary and compact, while Mesher and retained backend proof stay maintainer-facing.

### Done when
The public-secondary proof docs and subsystem handoffs describe the new deeper-reference story truthfully, and both the updated secondary-surfaces Node contract and compatibility proof-page verifier pass.
  - Estimate: 2h
  - Files: website/docs/docs/production-backend-proof/index.md, website/docs/docs/distributed/index.md, website/docs/docs/distributed-proof/index.md, website/docs/docs/web/index.md, website/docs/docs/databases/index.md, website/docs/docs/testing/index.md, website/docs/docs/concurrency/index.md, reference-backend/scripts/verify-production-proof-surface.sh, scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs
  - Verify: `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
`bash reference-backend/scripts/verify-production-proof-surface.sh`
- [x] **T03: Retargeted the clustered scaffold README, clustering skill, and their source-contract rails to the proof-page-first Mesher maintainer handoff.** — ### Why
The generated clustered scaffold README and the bundled clustering skill still encode the old public story: examples first, then direct repo-root `reference-backend/README.md`. That wording would keep new generated projects and agent guidance teaching the legacy backend path even after the docs pages are fixed. This task moves those generated and agent-facing surfaces onto the new deeper-reference contract and updates the existing source-level guardrails that literal-match them.

### Steps
1. Update the clustered scaffold README template in `compiler/mesh-pkg/src/scaffold.rs` so it keeps the public examples-first follow-on ladder but routes deeper backend proof through the public proof page plus maintainer-only Mesher/retained verifier language instead of a direct repo-root `reference-backend/README.md` handoff.
2. Rewrite `tools/skill/mesh/skills/clustering/SKILL.md` so it keeps the clustered scaffold and Todo examples as the public story, describes Mesher as the deeper maintained app for repo maintainers, and keeps the retained backend-only proof on named maintainer verifier commands instead of public fixture/runbook teaching.
3. Update `scripts/tests/verify-m049-s04-onboarding-contract.test.mjs` and `scripts/tests/verify-m048-s04-skill-contract.test.mjs` to fail closed on stale scaffold/skill wording and on any reintroduction of repo-root `reference-backend/README.md` as the public next step.
4. Rebind the existing Rust docs/scaffold contract tests in `compiler/meshc/tests/e2e_m047_s04.rs`, `compiler/meshc/tests/e2e_m047_s05.rs`, and `compiler/meshc/tests/e2e_m047_s06.rs` so they assert the new generated wording and deeper-reference split without dropping their named public-surface coverage.

### Must-Haves
- [ ] The clustered scaffold README template no longer teaches repo-root `reference-backend/README.md` as the public follow-on step.
- [ ] `tools/skill/mesh/skills/clustering/SKILL.md` keeps the examples-first public ladder and describes Mesher / retained backend proof as maintainer-only deeper surfaces.
- [ ] The onboarding and skill-contract Node tests fail closed on stale wording instead of relying on manual review.
- [ ] The older M047 docs/scaffold contract tests still run real tests and pin the new public/generated wording.

### Done when
Generated scaffold copy, bundled clustering skill guidance, and their existing source-contract rails all reflect the new examples-first / maintainer-only deeper-reference contract.
  - Estimate: 2h
  - Files: compiler/mesh-pkg/src/scaffold.rs, tools/skill/mesh/skills/clustering/SKILL.md, scripts/tests/verify-m049-s04-onboarding-contract.test.mjs, scripts/tests/verify-m048-s04-skill-contract.test.mjs, compiler/meshc/tests/e2e_m047_s04.rs, compiler/meshc/tests/e2e_m047_s05.rs, compiler/meshc/tests/e2e_m047_s06.rs
  - Verify: `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs`
`node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`
`cargo test -p meshc --test e2e_m047_s04 -- --nocapture`
`cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture`
`cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`
- [x] **T04: Fixed the immediate `e2e_m047_s05` compile regression and captured the remaining stale M047/M050 docs-rail failures that still block the new M051/S04 replay.** — ### Why
After the public copy, generated surfaces, and compatibility verifier all move, S04 still needs one authoritative acceptance surface and the older M050 verifier stack must stop encoding the old backend story. This task closes the slice with a named M051 rail that downstream S05 deletion work can reuse, while keeping the historical M050 wrapper paths green against the new contract.

### Steps
1. Create `compiler/meshc/tests/e2e_m051_s04.rs` as the slice-owned source/verifier contract target and `scripts/verify-m051-s04.sh` as the assembled replay for the retargeted docs/scaffold/skill stack.
2. Update `scripts/verify-m050-s01.sh`, `scripts/verify-m050-s02.sh`, and `scripts/verify-m050-s03.sh` so they replay the new proof-page semantics, public docs wording, and compatibility verifier behavior instead of the old repo-root backend story.
3. Update `compiler/meshc/tests/e2e_m050_s01.rs`, `compiler/meshc/tests/e2e_m050_s02.rs`, and `compiler/meshc/tests/e2e_m050_s03.rs` so their verifier-contract assertions match the new wrapper commands, public markers, and retained built-html bundle contents.
4. Make `scripts/verify-m051-s04.sh` publish `.tmp/m051-s04/verify/status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, `latest-proof-bundle.txt`, built-html snapshots, and copied contract artifacts so S05 inherits one stable acceptance surface.

### Must-Haves
- [ ] `compiler/meshc/tests/e2e_m051_s04.rs` runs more than zero real tests and archives the S04 contract surfaces it depends on.
- [ ] `bash scripts/verify-m051-s04.sh` is the authoritative slice replay and publishes the standard `.tmp/m051-s04/verify/` phase and bundle markers.
- [ ] The historical M050 wrapper scripts remain green but now encode the new public-secondary backend contract instead of the old repo-root backend story.
- [ ] The slice-owned replay proves the new docs/scaffold/skill contract together with the compatibility proof-page verifier and a real VitePress build.

### Failure Modes
| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `scripts/verify-m051-s04.sh` assembled replay | Stop on the first failing phase, write the phase marker, and keep the failing phase log plus artifact hint | Record the timeout in the phase log and fail closed; do not continue to later phases | Treat missing status/bundle markers or missing built-html snapshots as verifier drift |
| Historical M050 wrapper scripts | Fail if their command order, proof-page verifier semantics, or bundle-shape assertions no longer match the shipped contract | Respect explicit wrapper timeouts instead of retrying hiddenly | Treat missing test-count checks or stale built-html markers as contract failures |
| VitePress build output | Preserve the copied built HTML snapshots and summary file for postmortem | Fail on build timeout and keep the build log | Treat malformed or missing built HTML as an acceptance failure, not a soft warning |

### Load Profile
- **Shared resources**: `.tmp/m050-s01/verify/`, `.tmp/m050-s02/verify/`, `.tmp/m050-s03/verify/`, `.tmp/m051-s04/verify/`, and `website/docs/.vitepress/dist/`.
- **Per-operation cost**: one new Rust contract target, three wrapper-source contract updates, one assembled shell replay, and one VitePress build.
- **10x breakpoint**: the docs build and wrapper replays dominate first; repeated full-stack reruns get expensive before any code path becomes resource-sensitive.

### Negative Tests
- **Malformed inputs**: missing phase markers, stale proof-page command strings, wrong built-html file list, or missing copied contract artifacts.
- **Error paths**: wrapper scripts exit green but the named filters run 0 tests, the compatibility verifier path is missing, or the slice-owned replay skips the VitePress build/bundle assertions.
- **Boundary conditions**: legacy M050 paths stay alive and truthful while S04 publishes one new stable verifier surface for S05.

### Done when
The legacy M050 verifier stack matches the new contract and `compiler/meshc/tests/e2e_m051_s04.rs` plus `bash scripts/verify-m051-s04.sh` give S04 one authoritative retained acceptance surface.
  - Estimate: 2h
  - Files: compiler/meshc/tests/e2e_m051_s04.rs, scripts/verify-m051-s04.sh, scripts/verify-m050-s01.sh, scripts/verify-m050-s02.sh, scripts/verify-m050-s03.sh, compiler/meshc/tests/e2e_m050_s01.rs, compiler/meshc/tests/e2e_m050_s02.rs, compiler/meshc/tests/e2e_m050_s03.rs
  - Verify: `cargo test -p meshc --test e2e_m051_s04 -- --nocapture`
`bash scripts/verify-m051-s04.sh`
