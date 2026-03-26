---
id: T01
parent: S02
milestone: M034
provides: []
requires: []
affects: []
key_files: [".github/workflows/authoritative-live-proof.yml", "scripts/verify-m034-s02-workflows.sh", ".gsd/KNOWLEDGE.md"]
key_decisions: ["Keep the reusable workflow as the only workflow file that directly runs `bash scripts/verify-m034-s01.sh`; future lanes must call it rather than duplicate proof logic."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Verified the new verifier script parses, `bash scripts/verify-m034-s02-workflows.sh reusable` passes, and `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-live-proof.yml")'` parses the reusable workflow successfully. Also ran the current slice-level checks to record the expected intermediate-task failures: the full workflow verifier fails because `.github/workflows/authoritative-verification.yml` does not exist yet, and the full three-workflow YAML load fails for the same missing file."
completed_at: 2026-03-26T22:24:19.449Z
blocker_discovered: false
---

# T01: Added the reusable authoritative live-proof workflow and a repo-local verifier that enforces its Linux toolchain, secret wiring, proof entrypoint, and failure-artifact contract.

> Added the reusable authoritative live-proof workflow and a repo-local verifier that enforces its Linux toolchain, secret wiring, proof entrypoint, and failure-artifact contract.

## What Happened
---
id: T01
parent: S02
milestone: M034
key_files:
  - .github/workflows/authoritative-live-proof.yml
  - scripts/verify-m034-s02-workflows.sh
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Keep the reusable workflow as the only workflow file that directly runs `bash scripts/verify-m034-s01.sh`; future lanes must call it rather than duplicate proof logic.
duration: ""
verification_result: mixed
completed_at: 2026-03-26T22:24:19.450Z
blocker_discovered: false
---

# T01: Added the reusable authoritative live-proof workflow and a repo-local verifier that enforces its Linux toolchain, secret wiring, proof entrypoint, and failure-artifact contract.

**Added the reusable authoritative live-proof workflow and a repo-local verifier that enforces its Linux toolchain, secret wiring, proof entrypoint, and failure-artifact contract.**

## What Happened

Added `.github/workflows/authoritative-live-proof.yml` as the single reusable `workflow_call` unit that runs the real Mesh package-manager proof on Ubuntu x86_64 with the Linux LLVM 21 bootstrap reused from `release.yml`, explicit `MESH_PUBLISH_OWNER` / `MESH_PUBLISH_TOKEN` secret wiring into the proof step, and failure-only upload of `.tmp/m034-s01/verify/**`. Added `scripts/verify-m034-s02-workflows.sh` with a passing `reusable` contract check that enforces trigger shape, required secrets, toolchain setup, unchanged `bash scripts/verify-m034-s01.sh` handoff, failure-artifact retention, and the rule that no other workflow file directly invokes the S01 proof. Fixed the verifier's embedded Ruby entrypoint and taught it to tolerate Ruby YAML parsing GitHub Actions' bare `on:` key as boolean `true`.

## Verification

Verified the new verifier script parses, `bash scripts/verify-m034-s02-workflows.sh reusable` passes, and `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-live-proof.yml")'` parses the reusable workflow successfully. Also ran the current slice-level checks to record the expected intermediate-task failures: the full workflow verifier fails because `.github/workflows/authoritative-verification.yml` does not exist yet, and the full three-workflow YAML load fails for the same missing file.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `bash -n scripts/verify-m034-s02-workflows.sh` | 0 | ✅ pass | 9ms |
| 2 | `bash scripts/verify-m034-s02-workflows.sh reusable` | 0 | ✅ pass | 113ms |
| 3 | `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-live-proof.yml")'` | 0 | ✅ pass | 99ms |
| 4 | `bash scripts/verify-m034-s02-workflows.sh` | 1 | ❌ fail | 112ms |
| 5 | `ruby -e 'require "yaml"; %w[.github/workflows/authoritative-live-proof.yml .github/workflows/authoritative-verification.yml .github/workflows/release.yml].each { |f| YAML.load_file(f) }'` | 1 | ❌ fail | 94ms |


## Deviations

Added an `all` mode placeholder to `scripts/verify-m034-s02-workflows.sh` so the slice-level verifier stays truthfully red until T02 lands the caller workflow. This was not explicit in the task plan, but it avoids a misleading green full-slice check after only T01.

## Known Issues

`.github/workflows/authoritative-verification.yml` is not present yet, so the slice-level verifier and the full three-workflow YAML load both fail as expected on this intermediate task. `release.yml` is not gated on the reusable proof yet; that remains for T03.

## Files Created/Modified

- `.github/workflows/authoritative-live-proof.yml`
- `scripts/verify-m034-s02-workflows.sh`
- `.gsd/KNOWLEDGE.md`


## Deviations
Added an `all` mode placeholder to `scripts/verify-m034-s02-workflows.sh` so the slice-level verifier stays truthfully red until T02 lands the caller workflow. This was not explicit in the task plan, but it avoids a misleading green full-slice check after only T01.

## Known Issues
`.github/workflows/authoritative-verification.yml` is not present yet, so the slice-level verifier and the full three-workflow YAML load both fail as expected on this intermediate task. `release.yml` is not gated on the reusable proof yet; that remains for T03.
