---
estimated_steps: 4
estimated_files: 2
skills_used:
  - github-workflows
---

# T03: Gate tag releases on the shared proof and harden workflow permissions

**Slice:** S02 — Authoritative CI verification lane
**Milestone:** M034

## Description

Finish the slice by making tag releases depend on the same reusable live proof and by tightening `release.yml` permissions. Preserve the existing build matrices and packaging behavior, but ensure `Create Release` cannot run for `v*` tags until the reusable proof job has passed. The task also owns the final local verifier assertions across all workflows and the real GitHub Actions acceptance evidence that shows the shared proof reaching `verify-m034-s01: ok` on a trusted run.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| Reusable proof job in tag releases | Block `Create Release` entirely when the live proof fails instead of falling back to green artifact packaging. | Keep the release blocked until the proof job resolves or times out visibly. | Treat missing `needs:` wiring or a missing reusable-workflow reference as local-verifier failure. |
| Release workflow permissions | Fail closed by keeping non-release jobs read-only; do not leave workflow-wide write access in place for convenience. | N/A | Treat workflow-wide `contents: write` as permission drift that the local verifier must reject. |
| Existing build matrices and artifacts | Preserve current artifact builds for PR/main/tag paths while inserting the proof dependency only where release truth requires it. | Let upstream build jobs fail normally without masking proof failures. | Treat missing build prerequisites in the release graph as drift in the tag-gating contract. |

## Load Profile

- **Shared resources**: tag pushes, release artifacts, the reusable proof job, publish secrets, and the existing cross-platform build matrices.
- **Per-operation cost**: current meshc/meshpkg build matrices plus one authoritative proof run per tag release.
- **10x breakpoint**: release tags queued behind live proof or duplicated tag reruns would bottleneck first, so the dependency chain must stay linear and explicit.

## Negative Tests

- **Malformed inputs**: tag runs without secret mapping, workflow-wide write permissions, or a `release` job that no longer depends on the authoritative proof.
- **Error paths**: failed proof jobs must prevent release creation, and failed local verifier assertions must catch missing `needs:` or permission drift before push.
- **Boundary conditions**: non-tag pushes keep existing build behavior, while `v*` tags gain the extra authoritative proof requirement before publishing assets.

## Steps

1. Update `.github/workflows/release.yml` so tag runs call `.github/workflows/authoritative-live-proof.yml` and `Create Release` depends on that proof plus the existing `build` and `build-meshpkg` jobs.
2. Move `contents: write` from workflow scope to the `release` job so build and proof jobs stay read-only.
3. Extend `scripts/verify-m034-s02-workflows.sh` with the final cross-workflow assertions for shared reusable references, tag gating, and permission hardening.
4. Validate the lane end to end: run the local verifier, then capture trusted GitHub Actions evidence from a `workflow_dispatch` or same-repo push run that reaches `verify-m034-s01: ok` and from a `v*` run where `Create Release` is visibly downstream of the proof job.

## Must-Haves

- [ ] `release.yml` cannot create a GitHub Release for `v*` tags unless the authoritative proof has already passed.
- [ ] Workflow-wide write permissions are removed; only the actual release job keeps `contents: write`.
- [ ] The final local verifier script asserts the reusable proof, trusted-event lane, and tag-release gating as one coherent CI contract.
- [ ] Final acceptance captures real GitHub Actions evidence that the live proof runs to `verify-m034-s01: ok` on a trusted event.

## Verification

- `bash scripts/verify-m034-s02-workflows.sh`
- `ruby -e 'require "yaml"; %w[.github/workflows/authoritative-live-proof.yml .github/workflows/authoritative-verification.yml .github/workflows/release.yml].each { |f| YAML.load_file(f) }'`
- Trusted GitHub Actions evidence: a `workflow_dispatch` or same-repo `push` run shows `verify-m034-s01: ok`, and a `v*` run shows `Create Release` waiting on the authoritative proof job.

## Observability Impact

- Signals added/changed: the release graph now includes the reusable proof job, and local verification names permission or dependency drift explicitly.
- How a future agent inspects this: run `bash scripts/verify-m034-s02-workflows.sh`, inspect the release workflow graph, and open the retained `.tmp/m034-s01/verify/**` artifacts from the proof job.
- Failure state exposed: whether release creation was blocked by proof failure, secret/trust policy, or permission/wiring drift.

## Inputs

- `.github/workflows/release.yml` — current build and release graph that must keep its artifact behavior while gaining proof gating.
- `.github/workflows/authoritative-live-proof.yml` — reusable proof workflow that tag releases must call.
- `.github/workflows/authoritative-verification.yml` — trusted-event caller workflow whose proof contract must stay aligned with release gating.
- `scripts/verify-m034-s02-workflows.sh` — local verifier script to extend with final cross-workflow assertions.

## Expected Output

- `.github/workflows/release.yml` — release workflow gated on the shared live proof with write permissions scoped to the release job.
- `scripts/verify-m034-s02-workflows.sh` — final local verifier covering the reusable workflow, trusted-event lane, and tag-release gating.
