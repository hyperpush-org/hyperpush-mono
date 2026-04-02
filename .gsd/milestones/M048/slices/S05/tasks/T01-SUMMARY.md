---
id: T01
parent: S05
milestone: M048
provides: []
requires: []
affects: []
key_files: ["README.md", "website/docs/docs/tooling/index.md", "tools/editors/vscode-mesh/README.md", "scripts/tests/verify-m048-s05-contract.test.mjs"]
key_decisions: ["Public VS Code wording stays limited to same-file definition plus manifest-first override-entry hover/diagnostics proof.", "The S05 docs rail uses exact include/exclude markers so stale public wording fails closed instead of drifting silently."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "`node --test scripts/tests/verify-m048-s05-contract.test.mjs` passed after the doc updates. `npm --prefix website run build` passed. The slice-level partial check `bash scripts/verify-m048-s05.sh` currently fails with `No such file or directory`, which is expected for T01 because T02 owns that wrapper."
completed_at: 2026-04-02T18:32:44.889Z
blocker_discovered: false
---

# T01: Updated public Mesh docs for update and override-entry truth, and added the S05 fail-closed docs contract test.

> Updated public Mesh docs for update and override-entry truth, and added the S05 fail-closed docs contract test.

## What Happened
---
id: T01
parent: S05
milestone: M048
key_files:
  - README.md
  - website/docs/docs/tooling/index.md
  - tools/editors/vscode-mesh/README.md
  - scripts/tests/verify-m048-s05-contract.test.mjs
key_decisions:
  - Public VS Code wording stays limited to same-file definition plus manifest-first override-entry hover/diagnostics proof.
  - The S05 docs rail uses exact include/exclude markers so stale public wording fails closed instead of drifting silently.
duration: ""
verification_result: mixed
completed_at: 2026-04-02T18:32:44.891Z
blocker_discovered: false
---

# T01: Updated public Mesh docs for update and override-entry truth, and added the S05 fail-closed docs contract test.

**Updated public Mesh docs for update and override-entry truth, and added the S05 fail-closed docs contract test.**

## What Happened

Added a new fail-closed Node docs contract rail for the three public touchpoints, reproduced the missing/stale markers, then updated the root README, tooling docs, and VS Code README to document installer-backed updates, optional manifest entrypoints, truthful publish/archive behavior, bounded editor proof, and the new S05 verifier pointer without reintroducing the stale cross-file definition claim.

## Verification

`node --test scripts/tests/verify-m048-s05-contract.test.mjs` passed after the doc updates. `npm --prefix website run build` passed. The slice-level partial check `bash scripts/verify-m048-s05.sh` currently fails with `No such file or directory`, which is expected for T01 because T02 owns that wrapper.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `node --test scripts/tests/verify-m048-s05-contract.test.mjs` | 0 | ✅ pass | 1007ms |
| 2 | `npm --prefix website run build` | 0 | ✅ pass | 83900ms |
| 3 | `bash scripts/verify-m048-s05.sh` | 127 | ❌ fail | 73ms |


## Deviations

None.

## Known Issues

`scripts/verify-m048-s05.sh` does not exist yet, so the slice-level verifier remains red until T02 lands.

## Files Created/Modified

- `README.md`
- `website/docs/docs/tooling/index.md`
- `tools/editors/vscode-mesh/README.md`
- `scripts/tests/verify-m048-s05-contract.test.mjs`


## Deviations
None.

## Known Issues
`scripts/verify-m048-s05.sh` does not exist yet, so the slice-level verifier remains red until T02 lands.
