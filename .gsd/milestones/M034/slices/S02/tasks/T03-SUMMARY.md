---
id: T03
parent: S02
milestone: M034
provides: []
requires: []
affects: []
key_files: [".github/workflows/release.yml", "scripts/verify-m034-s02-workflows.sh", ".gsd/DECISIONS.md", ".gsd/KNOWLEDGE.md"]
key_decisions: ["Keep `release.yml` workflow-wide permissions read-only, grant `contents: write` only to `Create Release`, and require a tag-only reusable authoritative-proof job before publication."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Local verification passed: `bash -n scripts/verify-m034-s02-workflows.sh`, `bash scripts/verify-m034-s02-workflows.sh reusable`, `bash scripts/verify-m034-s02-workflows.sh caller`, `bash scripts/verify-m034-s02-workflows.sh release`, `bash scripts/verify-m034-s02-workflows.sh`, and Ruby YAML parses for all three workflow files all exited 0. GitHub-side acceptance evidence remains pending rollout: `gh run list --workflow authoritative-verification.yml --limit 1` returned 404 because the workflow is not yet on the remote default branch, and historical tag run `22562324587` still shows the legacy release graph without an authoritative proof job."
completed_at: 2026-03-26T22:43:14.258Z
blocker_discovered: false
---

# T03: Gated tag releases on the reusable authoritative live proof, scoped release write permissions to the publish job, and finished the cross-workflow verifier.

> Gated tag releases on the reusable authoritative live proof, scoped release write permissions to the publish job, and finished the cross-workflow verifier.

## What Happened
---
id: T03
parent: S02
milestone: M034
key_files:
  - .github/workflows/release.yml
  - scripts/verify-m034-s02-workflows.sh
  - .gsd/DECISIONS.md
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Keep `release.yml` workflow-wide permissions read-only, grant `contents: write` only to `Create Release`, and require a tag-only reusable authoritative-proof job before publication.
duration: ""
verification_result: mixed
completed_at: 2026-03-26T22:43:14.259Z
blocker_discovered: false
---

# T03: Gated tag releases on the reusable authoritative live proof, scoped release write permissions to the publish job, and finished the cross-workflow verifier.

**Gated tag releases on the reusable authoritative live proof, scoped release write permissions to the publish job, and finished the cross-workflow verifier.**

## What Happened

Updated `.github/workflows/release.yml` so tag releases call the reusable authoritative live-proof workflow, the workflow defaults to read-only permissions, and `Create Release` now depends on `build`, `build-meshpkg`, and the tag-only proof job before publishing assets. Rewrote `scripts/verify-m034-s02-workflows.sh` to replace the old T03 placeholder with a real `release` contract check plus a green `all` mode that enforces release trigger shape, proof reuse, explicit secret mapping, permission hardening, and release dependency wiring across all three workflow files. Also recorded the CI/security decision in `.gsd/DECISIONS.md` and added the GitHub CLI workflow-discovery gotcha to `.gsd/KNOWLEDGE.md` so future agents understand why remote acceptance evidence is unavailable until the workflows are pushed.

## Verification

Local verification passed: `bash -n scripts/verify-m034-s02-workflows.sh`, `bash scripts/verify-m034-s02-workflows.sh reusable`, `bash scripts/verify-m034-s02-workflows.sh caller`, `bash scripts/verify-m034-s02-workflows.sh release`, `bash scripts/verify-m034-s02-workflows.sh`, and Ruby YAML parses for all three workflow files all exited 0. GitHub-side acceptance evidence remains pending rollout: `gh run list --workflow authoritative-verification.yml --limit 1` returned 404 because the workflow is not yet on the remote default branch, and historical tag run `22562324587` still shows the legacy release graph without an authoritative proof job.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `bash -n scripts/verify-m034-s02-workflows.sh` | 0 | ✅ pass | 16ms |
| 2 | `bash scripts/verify-m034-s02-workflows.sh reusable` | 0 | ✅ pass | 125ms |
| 3 | `bash scripts/verify-m034-s02-workflows.sh caller` | 0 | ✅ pass | 109ms |
| 4 | `bash scripts/verify-m034-s02-workflows.sh release` | 0 | ✅ pass | 114ms |
| 5 | `bash scripts/verify-m034-s02-workflows.sh` | 0 | ✅ pass | 297ms |
| 6 | `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-live-proof.yml")'` | 0 | ✅ pass | 96ms |
| 7 | `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-verification.yml")'` | 0 | ✅ pass | 98ms |
| 8 | `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release.yml")'` | 0 | ✅ pass | 98ms |
| 9 | `ruby -e 'require "yaml"; %w[.github/workflows/authoritative-live-proof.yml .github/workflows/authoritative-verification.yml .github/workflows/release.yml].each { |f| YAML.load_file(f) }'` | 0 | ✅ pass | 98ms |
| 10 | `gh run list --workflow authoritative-verification.yml --limit 1` | 1 | ❌ fail | 415ms |
| 11 | `gh run view 22562324587 --json jobs,workflowName,displayTitle,event,headBranch,conclusion,status,url | jq -r '.workflowName + "\t" + .event + "\t" + .headBranch + "\t" + (.jobs | map(.name) | join(", "))'` | 0 | ✅ pass | 1323ms |


## Deviations

Could not complete the task-plan’s GitHub-side acceptance capture from the working tree alone because the authoritative workflow files are not yet present on the remote default branch GitHub is querying. Verified that with `gh` instead of inventing acceptance evidence.

## Known Issues

Remote acceptance evidence is still pending the next push of this task’s workflow changes. Until the updated workflow files exist on GitHub, `gh run list --workflow authoritative-verification.yml` returns 404 and historical `Release` tag runs still show the pre-T03 graph with no proof-gating job.

## Files Created/Modified

- `.github/workflows/release.yml`
- `scripts/verify-m034-s02-workflows.sh`
- `.gsd/DECISIONS.md`
- `.gsd/KNOWLEDGE.md`


## Deviations
Could not complete the task-plan’s GitHub-side acceptance capture from the working tree alone because the authoritative workflow files are not yet present on the remote default branch GitHub is querying. Verified that with `gh` instead of inventing acceptance evidence.

## Known Issues
Remote acceptance evidence is still pending the next push of this task’s workflow changes. Until the updated workflow files exist on GitHub, `gh run list --workflow authoritative-verification.yml` returns 404 and historical `Release` tag runs still show the pre-T03 graph with no proof-gating job.
