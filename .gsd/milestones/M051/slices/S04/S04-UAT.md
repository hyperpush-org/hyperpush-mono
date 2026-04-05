# S04: Retarget public docs, scaffold, and skills to the examples-first story — UAT

**Milestone:** M051
**Written:** 2026-04-04T19:48:25.825Z

# S04 UAT — Examples-first public docs, scaffold, and skill story

## Preconditions
- Repository is checked out at the completed M051/S04 state.
- `node`, `npm`, `cargo`, and `bash` are available on PATH.
- No local edits are hiding tracked file changes in the docs, scaffold, or skill surfaces being verified.

## Test Case 1 — Public first-contact path stays examples-first
1. Run `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`.
   - Expected: all 5 tests pass.
2. Open `README.md` and confirm the “Where to go next” section lists Clustered walkthrough, SQLite Todo starter, PostgreSQL Todo starter, then Production Backend Proof.
   - Expected: there is no direct public next step pointing to `reference-backend/README.md`.
3. Open `website/docs/docs/getting-started/index.md`.
   - Expected: the three starter commands (`meshc init --clustered`, SQLite Todo, PostgreSQL Todo) are all present and the follow-on list places Production Backend Proof after the starter/examples-first ladder.

## Test Case 2 — Public-secondary backend docs hand off to Mesher and retained verifier names
1. Run `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`.
   - Expected: all 4 tests pass.
2. Run `bash reference-backend/scripts/verify-production-proof-surface.sh`.
   - Expected: the verifier reports the production proof surface as verified.
3. Open `website/docs/docs/production-backend-proof/index.md`.
   - Expected: the page names `mesher/README.md`, `bash scripts/verify-m051-s01.sh`, and `bash scripts/verify-m051-s02.sh`, keeps the route/footer role intact, and does not teach `reference-backend/README.md` as the public next step.
4. Open `website/docs/docs/distributed/index.md` and `website/docs/docs/distributed-proof/index.md`.
   - Expected: they route readers through Production Backend Proof and the maintainer-only Mesher/retained-verifier handoff, not through repo-root backend teaching.

## Test Case 3 — Scaffold output and bundled skill match the new contract
1. Run `node --test scripts/tests/verify-m049-s04-onboarding-contract.test.mjs`.
   - Expected: all 6 tests pass.
2. Run `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`.
   - Expected: all 4 tests pass.
3. Inspect `compiler/mesh-pkg/src/scaffold.rs` for the clustered README template.
   - Expected: it points public readers to the Todo examples and Production Backend Proof, then names Mesher and `verify-m051-s01.sh` / `verify-m051-s02.sh` as maintainer-only surfaces.
4. Inspect `tools/skill/mesh/skills/clustering/SKILL.md`.
   - Expected: it explicitly forbids teaching `reference-backend/README.md` as the public next step and uses Production Backend Proof plus Mesher/retained verifier names instead.

## Test Case 4 — Historical docs rails still replay against the shipped copy
1. Run `cargo test -p meshc --test e2e_m047_s04 m047_s04_ -- --nocapture`.
   - Expected: 5 tests pass.
2. Run `cargo test -p meshc --test e2e_m047_s05 m047_s05_public_clustered_surfaces_use_source_first_names_and_todo_template -- --nocapture`.
   - Expected: 1 targeted test passes.
3. Run `cargo test -p meshc --test e2e_m047_s06 m047_s06_ -- --nocapture`.
   - Expected: 3 tests pass.
4. Inspect failures if any command goes red.
   - Expected edge-case behavior: drift should identify the exact stale public linkage (for example README/distributed/tooling/proof-page copy) rather than failing ambiguously.

## Test Case 5 — Authoritative S04 acceptance rail publishes the retained proof bundle
1. Run `cargo test -p meshc --test e2e_m051_s04 -- --nocapture`.
   - Expected: both tests pass.
2. Run `bash scripts/verify-m051-s04.sh`.
   - Expected: the script finishes with `verify-m051-s04: ok`.
3. Inspect `.tmp/m051-s04/verify/`.
   - Expected files: `status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, `latest-proof-bundle.txt`, `built-html/summary.json`.
4. Inspect `.tmp/m051-s04/verify/retained-proof-bundle/`.
   - Expected: it contains retained copies of the M050 wrapper bundles, the slice-owned M051 artifacts, built HTML snapshots, `e2e_m051_s04.rs`, and `verify-m051-s04.sh`.

## Edge Cases
- Reintroduce `reference-backend/README.md` into README, clustered scaffold guidance, or the clustering skill in a throwaway local edit, then rerun the matching Node contract.
  - Expected: the relevant contract fails closed and names the stale marker explicitly.
- Reintroduce `meshc test reference-backend` or `meshc fmt --check reference-backend` in Tooling.
  - Expected: the first-contact tooling contract fails closed.
- Remove the Mesher or retained-verifier markers from Production Backend Proof.
  - Expected: the proof-surface verifier and secondary-surface contract both fail closed.
