# S02: First-Contact Docs Rewrite — UAT

**Milestone:** M050
**Written:** 2026-04-04T03:11:56.595Z

# S02: First-Contact Docs Rewrite — UAT

**Milestone:** M050
**Written:** 2026-04-03

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S02 changed public documentation, docs-specific verification wrappers, and retained replay ordering. The shipped contract is therefore proven by source-level docs assertions, built-site HTML snapshots, and one green assembled onboarding replay.

## Preconditions

- Run from the repo root.
- Node/npm dependencies for `website/` and `tools/editors/vscode-mesh/` are installed.
- Rust builds succeed in the current workspace.
- Docker, `nvim`, and the retained VS Code smoke prerequisites are available if you plan to run the full assembled replay.

## Smoke Test

Run `bash scripts/verify-m050-s02.sh`.

**Expected:** the command exits 0, prints `verify-m050-s02: ok`, and writes `.tmp/m050-s02/verify/` with `status.txt=ok`, a populated `phase-report.txt`, and copied HTML snapshots for Getting Started, Clustered Example, and Tooling.

## Test Cases

### 1. README and Getting Started present the same starter chooser

1. Open `README.md` and `website/docs/docs/getting-started/index.md`.
2. Confirm both surfaces mention `meshc init --clustered`, `meshc init --template todo-api --db sqlite`, and `meshc init --template todo-api --db postgres`.
3. Confirm SQLite is described as the honest local starter, Postgres as the shared/deployable path, and the clustered next step appears before the production-proof next step.
4. **Expected:** both entrypoints tell the same evaluator-facing story and use `https://github.com/snowdamiz/mesh-lang.git`.

### 2. Clustered Example stays scaffold-first and hands proof discoverability off to Distributed Proof

1. Open `website/docs/docs/getting-started/clustered-example/index.md`.
2. Confirm the page starts with `meshc init --clustered hello_cluster`, keeps the route-free `@cluster pub fn add() -> Int do` example, and preserves the runtime CLI order `meshc cluster status` -> `meshc cluster continuity` -> `meshc cluster diagnostics`.
3. Confirm `## After the scaffold, pick the follow-on starter` lists the SQLite starter, the Postgres starter, and `reference-backend/README.md`.
4. Confirm the page links to `/docs/distributed-proof/` instead of listing `scripts/verify-m047-s04.sh` or `scripts/verify-m047-s06.sh` inline.
5. **Expected:** Clustered Example remains a first-contact scaffold walkthrough, not a proof-map page.

### 3. Tooling exposes the new docs verifier before the assembled onboarding verifier

1. Open `website/docs/docs/tooling/index.md`.
2. Confirm install/update and project-creation guidance appears before the release/proof runbook sections.
3. Confirm the page includes `bash scripts/verify-m050-s02.sh` before `bash scripts/verify-m049-s05.sh`.
4. **Expected:** Tooling reinforces the same first-contact story and makes the S02 docs verifier discoverable ahead of the broader assembled replay.

### 4. Built-site verifier captures HTML evidence for the public first-contact pages

1. Run `bash scripts/verify-m050-s02.sh`.
2. Inspect `.tmp/m050-s02/verify/phase-report.txt` and `.tmp/m050-s02/verify/built-html/summary.json`.
3. Confirm the copied files exist: `getting-started.index.html`, `clustered-example.index.html`, and `tooling.index.html`.
4. **Expected:** the phase report includes `first-contact-contract`, `docs-build`, `retain-built-html`, `built-html`, and `m050-s02-bundle-shape` as passed phases.

### 5. The assembled onboarding replay runs the M050 docs preflights before retained historical rails

1. Run `bash scripts/verify-m049-s05.sh`.
2. Inspect `.tmp/m049-s05/verify/phase-report.txt`.
3. Confirm the first phases after `init` are `m050-s01-preflight`, `m050-s02-preflight`, and `m049-s04-onboarding-contract`.
4. **Expected:** the replay finishes with `verify-m049-s05: ok` and writes `.tmp/m049-s05/verify/retained-proof-bundle/`.

## Edge Cases

### Clustered Example does not regress into a direct proof-rail page

1. Run `rg -n 'scripts/verify-m047-s04.sh|scripts/verify-m047-s06.sh|scripts/verify-m046-s06.sh|scripts/verify-m045-s05.sh' website/docs/docs/getting-started/clustered-example/index.md`.
2. Run `rg -n 'scripts/verify-m047-s04.sh' website/docs/docs/distributed-proof/index.md website/docs/docs/tooling/index.md`.
3. **Expected:** the first command returns no matches and the second command finds retained M047 rail references on Distributed Proof and/or Tooling.

### Fallback local Postgres metadata drift does not turn docs work red

1. Ensure `.tmp/m049-s01/local-postgres/connection.env` and `.tmp/m049-s01/local-postgres/container-meta.txt` exist.
2. If the local fallback container is stopped or its published port has changed, rerun `bash scripts/verify-m049-s05.sh`.
3. Inspect `.tmp/m049-s05/verify/m049-s01-env-preflight.*` plus the phase report.
4. **Expected:** the wrapper refreshes the fallback source and still reaches a green `m049-s01-e2e` phase instead of failing immediately with `failed to connect to admin database: connection refused`.

## Failure Signals

- Missing or reordered starter commands in README / Getting Started.
- `Clustered Example` containing direct retained rail commands instead of only linking to `/docs/distributed-proof/`.
- `bash scripts/verify-m050-s02.sh` missing `docs-build`, `built-html`, or `m050-s02-bundle-shape` in `.tmp/m050-s02/verify/phase-report.txt`.
- `bash scripts/verify-m049-s05.sh` failing before or during `m050-s02-preflight`.
- `bash scripts/verify-m036-s01.sh` failing because a docs-backed syntax corpus case no longer selects a real interpolation snippet.

## Requirements Proved By This UAT

- None explicitly reclassified in this slice closeout. This UAT proves the M050/S02 first-contact docs contract and its retained replay integration.

## Not Proven By This UAT

- The remaining S03 rewrite of deeper proof-heavy pages and two-layer truth across all secondary docs surfaces.
- Any live deployment or production-hosting behavior beyond the retained verifier wrappers already exercised by `bash scripts/verify-m049-s05.sh`.
- Resilience against arbitrary future Markdown line moves in docs-backed syntax corpus cases; the current protection is explicit corpus maintenance plus `bash scripts/verify-m036-s01.sh`.

## Notes for Tester

- Use `bash scripts/verify-m050-s02.sh` first whenever you suspect first-contact docs drift. It is much cheaper and more diagnostic than the full assembled `bash scripts/verify-m049-s05.sh` replay.
- If the full replay fails in `m049-s01-e2e`, check `.tmp/m049-s05/verify/m049-s01-env-preflight.*` before touching the docs. That failure can be local fallback Postgres metadata drift, not a docs regression.
- If a retained grammar rail fails after editing docs examples, inspect `scripts/fixtures/m036-s01-syntax-corpus.json` before changing the shared grammar.
