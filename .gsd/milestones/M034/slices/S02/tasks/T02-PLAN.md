---
estimated_steps: 4
estimated_files: 2
skills_used:
  - github-workflows
---

# T02: Add the trusted-event authoritative verification lane

**Slice:** S02 — Authoritative CI verification lane
**Milestone:** M034

## Description

Give the repo a named CI lane that reruns the reusable live proof on the events that can safely hold publish secrets. This workflow should cover same-repo pull requests, pushes to `main`, manual dispatch, and a bounded weekly schedule, while failing closed on forks. The executor should make the trust policy explicit in YAML comments and in the local verifier so later edits cannot quietly widen the secret boundary or delete the drift-monitor run.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| GitHub `pull_request` trust metadata | Fail closed and skip the live proof when the PR head repo is not trusted. | N/A | Treat missing or ambiguous repo metadata as untrusted and skip instead of injecting secrets. |
| Reusable workflow call contract | Fail the caller job visibly if the reusable workflow path or required secrets drift. | Let the reusable workflow timeout/fail visibly; do not fall back to a green build-only status. | Treat missing secret mappings or job `uses:` wiring as local-verifier failures. |
| Scheduled/manual triggers | Keep manual dispatch available even if schedule cadence changes; do not rely on tag pushes for drift detection. | Serialize overlapping runs rather than stacking multiple live proofs on the same ref. | Treat a missing `schedule` or `workflow_dispatch` stanza as CI-lane drift. |

## Load Profile

- **Shared resources**: GitHub Actions concurrency groups, runner minutes, the real publish proof package namespace, and the reusable workflow’s secret inputs.
- **Per-operation cost**: one live proof run for each trusted push/PR/manual/scheduled event.
- **10x breakpoint**: same-ref bursts and schedule overlaps would pile up first, so the caller workflow should use explicit concurrency and avoid unnecessary matrix fan-out.

## Negative Tests

- **Malformed inputs**: fork PRs, missing secret mappings, missing schedule stanza, or a caller workflow that invokes `pull_request_target`.
- **Error paths**: same-repo PRs should fail red when the proof fails, while fork PRs should skip the live proof rather than executing with secrets.
- **Boundary conditions**: `push` to `main`, `workflow_dispatch`, and scheduled drift-monitor runs all invoke the same reusable proof job and not a forked copy.

## Steps

1. Add `.github/workflows/authoritative-verification.yml` with `pull_request`, `push` to `main`, `workflow_dispatch`, and a weekly `schedule`, plus read-only permissions and concurrency keyed to workflow plus ref.
2. Call `.github/workflows/authoritative-live-proof.yml` only for trusted events: same-repo PRs, `main` pushes, manual dispatches, and scheduled runs.
3. Keep fork PRs on secret-free build checks and never use `pull_request_target`; add a short inline explanation so later maintainers understand why the live proof is skipped for forks.
4. Extend `scripts/verify-m034-s02-workflows.sh` with a `caller` mode that asserts the trigger set, explicit secret mapping, fork-skip condition, concurrency, and the absence of `pull_request_target`.

## Must-Haves

- [ ] The repo has a named authoritative verification workflow that reruns the S01 proof on trusted PR/main/manual/scheduled paths.
- [ ] Fork PRs do not receive live-publish secrets and are not upgraded to `pull_request_target` execution.
- [ ] The local verifier rejects drift in trigger policy, secret mapping, or concurrency before CI runs.

## Verification

- `bash scripts/verify-m034-s02-workflows.sh caller`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-verification.yml")'`

## Observability Impact

- Signals added/changed: a named `Authoritative verification` workflow with visible skip-vs-run behavior on trusted and untrusted events.
- How a future agent inspects this: inspect the caller workflow/job graph or run `bash scripts/verify-m034-s02-workflows.sh caller`.
- Failure state exposed: whether a run was skipped for trust policy, failed in the reusable proof, or drifted in trigger/secret wiring.

## Inputs

- `.github/workflows/authoritative-live-proof.yml` — reusable proof workflow from T01 that the caller must reuse instead of copying.
- `scripts/verify-m034-s02-workflows.sh` — local verifier script to extend with trusted-event assertions.
- `.github/workflows/release.yml` — existing workflow conventions for names, permissions, and concurrency style.

## Expected Output

- `.github/workflows/authoritative-verification.yml` — named trusted-event caller workflow for same-repo PRs, `main`, manual runs, and schedule.
- `scripts/verify-m034-s02-workflows.sh` — local contract verifier with a `caller` mode for trigger, trust, and concurrency checks.
