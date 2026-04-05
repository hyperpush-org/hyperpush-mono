# S03: Secondary Docs Surfaces & Two-Layer Truth — UAT

**Milestone:** M050
**Written:** 2026-04-04T04:46:02.465Z

# S03: Secondary Docs Surfaces & Two-Layer Truth — UAT

**Milestone:** M050
**Written:** 2026-04-03

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: this slice changed public docs surfaces and retained docs verifiers, so the authoritative proof is source contracts, built HTML snapshots, and retained wrapper bundles rather than a long-lived runtime service.

## Preconditions

- The repo contains the S03 docs and verifier changes.
- Website dependencies are installed so `npm --prefix website run build` succeeds.
- For the assembled `bash scripts/verify-m049-s05.sh` replay, the local fallback Postgres source in `.tmp/m049-s01/local-postgres/` is available or refreshable by the wrapper.

## Smoke Test

1. Run `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`.
2. **Expected:** all four tests pass, proving `Distributed Proof` owns the clustered verifier map, `Distributed Actors` and `Tooling` only hand off, and the supporting guides route through `Production Backend Proof` before the deeper runbook.

## Test Cases

### 1. Distributed Proof owns the clustered verifier ledger

1. Run `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`.
2. Run `cargo test -p meshc --test e2e_m047_s05 -- --nocapture`.
3. Run `cargo test -p meshc --test e2e_m047_s06 -- --nocapture`.
4. **Expected:** all three test targets pass and keep the retained M047 docs surfaces aligned with the new split: named clustered rails stay on `website/docs/docs/distributed-proof/index.md`, while `website/docs/docs/distributed/index.md` and `website/docs/docs/tooling/index.md` do not regain duplicate rail lists.

### 2. Production Backend Proof stays compact and is routed from subsystem guides

1. Run `bash reference-backend/scripts/verify-production-proof-surface.sh`.
2. Inspect `website/docs/docs/production-backend-proof/index.md`, `website/docs/docs/web/index.md`, `website/docs/docs/databases/index.md`, `website/docs/docs/testing/index.md`, and `website/docs/docs/concurrency/index.md`.
3. **Expected:** the verifier passes; Production Backend Proof keeps only the canonical backend command list and recovery fields; the supporting guides link to `/docs/production-backend-proof/` before the deeper `reference-backend/README.md` runbook.

### 3. The dedicated S03 verifier produces retained source/build evidence

1. Run `cargo test -p meshc --test e2e_m050_s03 -- --nocapture`.
2. Run `bash scripts/verify-m050-s03.sh`.
3. Inspect `.tmp/m050-s03/verify/status.txt`, `.tmp/m050-s03/verify/phase-report.txt`, and `.tmp/m050-s03/verify/built-html/summary.json`.
4. **Expected:** the Rust contract passes; the shell verifier ends with `status.txt` = `ok`; `phase-report.txt` shows `secondary-surfaces-contract`, `production-proof-surface`, `docs-build`, `retain-built-html`, `built-html`, and `m050-s03-bundle-shape` all passed; `built-html/summary.json` contains marker/link maps for `distributed`, `distributed-proof`, and `production-backend-proof`.

### 4. The assembled M049 replay retains the S03 bundle in the right order

1. Run `node --test scripts/tests/verify-m049-s05-contract.test.mjs`.
2. Run `cargo test -p meshc --test e2e_m049_s05 -- --nocapture`.
3. Run `bash scripts/verify-m049-s05.sh`.
4. Inspect `.tmp/m049-s05/verify/phase-report.txt` and `.tmp/m049-s05/verify/retained-proof-bundle/retained-m050-s03-verify/phase-report.txt`.
5. **Expected:** both contract tests pass; the assembled replay shows `m050-s03-preflight` immediately after `m050-s02-preflight`; the retained proof bundle contains `retained-m050-s03-verify`; the copied S03 phase report still shows all S03 phases passed.

## Edge Cases

### Distributed Actors or Tooling tries to reclaim named clustered rails

1. In a scratch copy, append `bash scripts/verify-m047-s04.sh` or `cargo test -p meshc --test e2e_m047_s07 -- --nocapture` to `website/docs/docs/distributed/index.md` or `website/docs/docs/tooling/index.md`.
2. Run `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`.
3. **Expected:** the test fails closed, reporting that the mutated page still contains stale clustered-rail text.

### Production Backend Proof drifts back into first-contact onboarding

1. In a scratch copy, insert `meshc init --clustered` or installer commands into `website/docs/docs/production-backend-proof/index.md`.
2. Run `bash reference-backend/scripts/verify-production-proof-surface.sh`.
3. **Expected:** the verifier fails closed on first-contact/install drift instead of treating the backend proof page as another onboarding page.

## Failure Signals

- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs` reports missing page-role markers, stale clustered-rail markers on Distributed Actors/Tooling, or missing `/docs/production-backend-proof/` handoffs.
- `bash reference-backend/scripts/verify-production-proof-surface.sh` reports sidebar drift, lost runbook parity, or onboarding/install text leaking into Production Backend Proof.
- `.tmp/m050-s03/verify/phase-report.txt` stops before `built-html` or `m050-s03-bundle-shape`.
- `.tmp/m049-s05/verify/phase-report.txt` is missing `m050-s03-preflight` or the retained bundle lacks `retained-m050-s03-verify`.

## Requirements Proved By This UAT

- R117 — The public docs stay evaluator-facing and keep proof-maze material off the main path while retaining a bounded proof map.
- R118 — Clustered guidance now has one primary evaluator path and a clear split between low-level distributed primitives, clustered proof, and backend proof surfaces.

## Not Proven By This UAT

- M051’s future `mesher` replacement of `reference-backend`; this slice still treats `reference-backend/README.md` as the deeper backend runbook.
- Landing-page or packages-site alignment; those remain later public-surface work outside the docs-secondary-surface slice.

## Notes for Tester

- `bash scripts/verify-m049-s05.sh` is the authoritative retained replay but it is much slower than the source/Rust preflights because it replays older retained rails and may refresh the local fallback Postgres source.
- If the dedicated S03 shell verifier fails after the docs build, inspect `.tmp/m050-s03/verify/built-html/summary.json` before reopening the broader M049 wrapper.
