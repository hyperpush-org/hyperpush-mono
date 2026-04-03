---
estimated_steps: 14
estimated_files: 5
skills_used:
  - bash-scripting
  - vitepress
  - test
---

# T03: Add the M050 graph verifier and wire it into the retained wrapper contracts

**Slice:** S01 — Onboarding Graph & Retained Rail Reset
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

## Inputs

- `scripts/tests/verify-m050-s01-onboarding-graph.test.mjs`
- `compiler/meshc/tests/e2e_m047_s04.rs`
- `compiler/meshc/tests/e2e_m047_s06.rs`
- `scripts/verify-m047-s04.sh`
- `scripts/verify-m047-s06.sh`
- `reference-backend/scripts/verify-production-proof-surface.sh`
- `scripts/verify-m049-s05.sh`
- `scripts/tests/verify-m049-s05-contract.test.mjs`
- `compiler/meshc/tests/e2e_m049_s05.rs`

## Expected Output

- `scripts/verify-m050-s01.sh`
- `compiler/meshc/tests/e2e_m050_s01.rs`
- `scripts/verify-m049-s05.sh`
- `scripts/tests/verify-m049-s05-contract.test.mjs`
- `compiler/meshc/tests/e2e_m049_s05.rs`
- `.tmp/m050-s01/verify/phase-report.txt`
- `.tmp/m050-s01/verify/latest-proof-bundle.txt`

## Verification

- `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`
- `node --test scripts/tests/verify-m049-s05-contract.test.mjs`
- `cargo test -p meshc --test e2e_m049_s05 -- --nocapture`
- `bash scripts/verify-m050-s01.sh`

## Observability Impact

- Signals added/changed: `.tmp/m050-s01/verify/{status.txt,current-phase.txt,phase-report.txt,full-contract.log,latest-proof-bundle.txt}` plus copied built-HTML evidence.
- How a future agent inspects this: read the phase report and retained bundle to localize whether drift is in source graph, retained contract, docs build, or built-site HTML.
- Failure state exposed: a named phase plus the offending built HTML or contract file instead of a generic docs-build failure.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| `scripts/verify-m050-s01.sh` phase runner | Fail on the named phase and preserve `status.txt`, `current-phase.txt`, `phase-report.txt`, and `full-contract.log` for diagnosis. | Kill the long-running phase, mark it failed, and retain the partial verify directory. | Reject missing bundle pointers, missing built-HTML evidence, or missing retained artifacts as verifier drift. |
| built-site assertions over `website/docs/.vitepress/dist/docs/*.html` | Stop when the build output does not prove `Getting Started → Clustered Example`, when `Clustered Example` still self-links, or when proof pages still render a footer. | Preserve the docs-build log and any copied HTML snapshot rather than silently skipping the assertion. | Treat missing or malformed HTML evidence as failure instead of assuming a green `npm --prefix website run build` is sufficient. |
| retained M049 wrapper source contracts | Fail the source-level contract tests if the wrapper stops acknowledging the new preflight or reorders it behind heavier runtime/sample rails. | N/A — source test only. | Reject malformed phase/order expectations instead of accepting an unpinned wrapper change. |

## Load Profile

- **Shared resources**: `.tmp/m050-s01/verify`, built VitePress HTML under `website/docs/.vitepress/dist/docs/`, and the active M049 wrapper source files.
- **Per-operation cost**: one Node test, a few source-level Rust tests, one docs build, one built-HTML scan, and retained artifact copying.
- **10x breakpoint**: docs build time and retained artifact churn dominate first; the verifier should stay env-free and must not fan out into Postgres-backed runtime replays just to prove the graph contract.

## Negative Tests

- **Malformed inputs**: missing built HTML files, empty verify directories, missing phase markers, or wrapper source that mentions `scripts/verify-m050-s01.sh` without actually placing it in the expected order.
- **Error paths**: docs build succeeds but built HTML still shows early proof routing, proof pages render a footer again, or the M049 wrapper drops the new preflight.
- **Boundary conditions**: the verifier stays env-free, the built HTML is checked after a real `npm --prefix website run build`, and retained bundle pointers resolve inside `.tmp/m050-s01/verify/`.
