---
id: T03
parent: S01
milestone: M051
provides: []
requires: []
affects: []
key_files: ["mesher/README.md", "scripts/verify-m051-s01.sh", "compiler/meshc/tests/e2e_m051_s01.rs", "mesher/.env.example", "mesher/main.mpl", "mesher/migrations/20260226000000_seed_default_org.mpl", ".gsd/KNOWLEDGE.md", ".gsd/DECISIONS.md"]
key_decisions: ["Use `mesher/README.md` plus `scripts/verify-m051-s01.sh` as the canonical Mesher maintainer surface, and keep public README/VitePress docs untouched until the later docs slice.", "Verifier self-audits should check the concrete `run_expect_success` phase-to-command mapping and retained bundle shape instead of raw denylist substring absence."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Ran `bash -n scripts/verify-m051-s01.sh` and the two new contract-focused filters in `cargo test -p meshc --test e2e_m051_s01 ...` while iterating. The authoritative gate was `bash scripts/verify-m051-s01.sh`, which passed in 114.1s and completed `m051-s01-package-tests`, `m051-s01-build`, `m051-s01-contract`, `m051-s01-e2e`, `retain-m051-s01-artifacts`, and `m051-s01-bundle-shape`. The verifier finished with `.tmp/m051-s01/verify/status.txt=ok`, `.tmp/m051-s01/verify/current-phase.txt=complete`, and `latest-proof-bundle.txt` pointing at `.tmp/m051-s01/verify/retained-proof-bundle`."
completed_at: 2026-04-04T07:55:35.473Z
blocker_discovered: false
---

# T03: Published a package-local Mesher maintainer runbook and fail-closed verifier tied to the real Mesher runtime rail.

> Published a package-local Mesher maintainer runbook and fail-closed verifier tied to the real Mesher runtime rail.

## What Happened
---
id: T03
parent: S01
milestone: M051
key_files:
  - mesher/README.md
  - scripts/verify-m051-s01.sh
  - compiler/meshc/tests/e2e_m051_s01.rs
  - mesher/.env.example
  - mesher/main.mpl
  - mesher/migrations/20260226000000_seed_default_org.mpl
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
key_decisions:
  - Use `mesher/README.md` plus `scripts/verify-m051-s01.sh` as the canonical Mesher maintainer surface, and keep public README/VitePress docs untouched until the later docs slice.
  - Verifier self-audits should check the concrete `run_expect_success` phase-to-command mapping and retained bundle shape instead of raw denylist substring absence.
duration: ""
verification_result: passed
completed_at: 2026-04-04T07:55:35.474Z
blocker_discovered: false
---

# T03: Published a package-local Mesher maintainer runbook and fail-closed verifier tied to the real Mesher runtime rail.

**Published a package-local Mesher maintainer runbook and fail-closed verifier tied to the real Mesher runtime rail.**

## What Happened

Published a package-local Mesher maintainer runbook and paired it with a fail-closed verifier tied to the real Mesher runtime rail. `mesher/README.md` now documents the repo-root migrate/build/run loop, the full env contract from `mesher/.env.example`, the seeded default org/project/API-key facts, the readiness check, the real `POST /api/v1/events` smoke, and the issues/storage readback path already proven by the live Mesher e2e target. `scripts/verify-m051-s01.sh` writes named phase logs under `.tmp/m051-s01/verify/`, replays Mesher package tests, build, and the dedicated `e2e_m051_s01` runtime rail, then copies the fresh `.tmp/m051-s01/*` artifacts into a retained proof bundle and validates the bundle shape. The Rust target gained contract tests that pin the README and verifier surface in the same test target as the live runtime proof, and the package-local `.env.example`, startup comments, and seed migration comments were aligned to the same maintainer contract. I also recorded the maintainer-surface boundary in `DECISIONS.md` and the verifier self-audit gotcha in `KNOWLEDGE.md`.

## Verification

Ran `bash -n scripts/verify-m051-s01.sh` and the two new contract-focused filters in `cargo test -p meshc --test e2e_m051_s01 ...` while iterating. The authoritative gate was `bash scripts/verify-m051-s01.sh`, which passed in 114.1s and completed `m051-s01-package-tests`, `m051-s01-build`, `m051-s01-contract`, `m051-s01-e2e`, `retain-m051-s01-artifacts`, and `m051-s01-bundle-shape`. The verifier finished with `.tmp/m051-s01/verify/status.txt=ok`, `.tmp/m051-s01/verify/current-phase.txt=complete`, and `latest-proof-bundle.txt` pointing at `.tmp/m051-s01/verify/retained-proof-bundle`.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `bash scripts/verify-m051-s01.sh` | 0 | ✅ pass | 114100ms |


## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `mesher/README.md`
- `scripts/verify-m051-s01.sh`
- `compiler/meshc/tests/e2e_m051_s01.rs`
- `mesher/.env.example`
- `mesher/main.mpl`
- `mesher/migrations/20260226000000_seed_default_org.mpl`
- `.gsd/KNOWLEDGE.md`
- `.gsd/DECISIONS.md`


## Deviations
None.

## Known Issues
None.
