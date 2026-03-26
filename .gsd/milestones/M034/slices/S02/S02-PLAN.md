# S02: Authoritative CI verification lane

**Goal:** Promote the S01 live registry verifier into an authoritative GitHub Actions lane by introducing one reusable proof workflow, running it on trusted PR/main/manual/scheduled paths, and blocking tag releases on the same real package-manager proof before assets publish.
**Demo:** After this: PR and release verification rerun the real Mesh proof surfaces, including the package-manager path, instead of stopping at artifact builds.

## Tasks
- [x] **T01: Added the reusable authoritative live-proof workflow and a repo-local verifier that enforces its Linux toolchain, secret wiring, proof entrypoint, and failure-artifact contract.** — The slice only becomes trustworthy once one GitHub Actions unit owns the real package-manager proof instead of scattering publish/install logic across multiple YAML files. Add a reusable Linux x86_64 workflow that installs Rust and LLVM 21, consumes explicit Mesh publish secrets, runs `bash scripts/verify-m034-s01.sh`, and uploads verifier artifacts on failure. Pair it with a repo-local verifier script so later workflow edits can be checked mechanically instead of by eyeballing YAML.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| GitHub runner toolchain bootstrap | Fail the job before any publish attempt and leave the failing setup step visible in the reusable workflow logs. | Stop the proof job and surface that the failure happened during LLVM/Rust setup, not during registry proof. | Treat missing LLVM prefix or missing Rust toolchain as contract drift and fail the local verifier. |
| `scripts/verify-m034-s01.sh` | Fail fast and upload `.tmp/m034-s01/verify/**` so the first broken proof phase stays inspectable. | Let the Actions job fail visibly; do not retry inside YAML with the same run state. | Treat a missing `verify-m034-s01: ok` terminal line as proof failure. |
| Artifact upload for failure diagnostics | Keep the proof red on upload errors rather than silently losing postmortem evidence. | Prefer preserving the primary proof failure over hanging indefinitely on artifact retention. | Treat missing verifier-output paths as drift in the reusable workflow contract. |

## Load Profile

- **Shared resources**: GitHub-hosted Ubuntu runner minutes, Cargo/LLVM download caches, the real publish token, and the dedicated proof package namespace.
- **Per-operation cost**: one LLVM bootstrap, one Rust toolchain setup, one local `cargo build`, and one live publish/install/download proof run.
- **10x breakpoint**: repeated cold-start toolchain installs and overlapping proof runs would hurt first, so the workflow must stay single-host and cache-friendly.

## Negative Tests

- **Malformed inputs**: missing `MESH_PUBLISH_OWNER`, missing `MESH_PUBLISH_TOKEN`, missing `scripts/verify-m034-s01.sh`, or a reusable workflow that omits the Linux toolchain setup.
- **Error paths**: failed toolchain bootstrap, failed live proof, and failed artifact upload should all stop the job without claiming partial success.
- **Boundary conditions**: the reusable workflow stays thin and shells out to the S01 verifier unchanged instead of re-implementing publish/install assertions in YAML.

## Steps

1. Add `.github/workflows/authoritative-live-proof.yml` as a reusable `workflow_call` workflow with a stable job name, Linux x86_64 runner, Rust setup, and the LLVM 21 bootstrap needed by `meshpkg`/`meshc`.
2. Pass `MESH_PUBLISH_OWNER` and `MESH_PUBLISH_TOKEN` only through Actions secrets/env, run `bash scripts/verify-m034-s01.sh` unchanged, and upload `.tmp/m034-s01/verify/**` when the proof fails so downstream debugging has the same artifacts S01 established locally.
3. Add `scripts/verify-m034-s02-workflows.sh` with a `reusable` mode that parses the workflow YAML and asserts the reusable workflow calls `scripts/verify-m034-s01.sh`, wires the Linux toolchain, and retains failure artifacts.
4. Keep the reusable workflow policy-free: no PR/tag triggers, no duplicate publish/install assertions, and no secret handling outside the explicit reusable-workflow inputs.

## Must-Haves

- [ ] `.github/workflows/authoritative-live-proof.yml` is the only GitHub Actions definition that knows how to run the live Mesh package-manager proof.
- [ ] The reusable workflow reuses `scripts/verify-m034-s01.sh` unchanged and retains `.tmp/m034-s01/verify/**` for failure diagnosis.
- [ ] A local verifier script can mechanically reject drift in the reusable workflow contract before CI runs.

## Verification

- `bash scripts/verify-m034-s02-workflows.sh reusable`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-live-proof.yml")'`
  - Estimate: 2h
  - Files: .github/workflows/authoritative-live-proof.yml, scripts/verify-m034-s02-workflows.sh, scripts/verify-m034-s01.sh, .github/workflows/release.yml
  - Verify: bash scripts/verify-m034-s02-workflows.sh reusable
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-live-proof.yml")'
- [ ] **T02: Add the trusted-event authoritative verification lane** — Once the reusable proof exists, give the repo a named CI lane that runs it on trusted events and fails closed on fork PR secret boundaries. This workflow should cover same-repo pull requests, pushes to `main`, manual dispatch, and a bounded schedule, while explicitly avoiding `pull_request_target` and other unsafe ways to run checked-out PR code with publish secrets. Extend the local verifier so future edits cannot silently widen the trust boundary or remove the scheduled drift monitor.

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

1. Add `.github/workflows/authoritative-verification.yml` with `pull_request`, `push` to `main`, `workflow_dispatch`, and a weekly `schedule`, plus read-only permissions and concurrency keyed to the workflow/ref.
2. Call `.github/workflows/authoritative-live-proof.yml` only for trusted events: same-repo PRs, `main` pushes, manual dispatches, and scheduled runs; keep fork PRs on secret-free build checks and never use `pull_request_target`.
3. Extend `scripts/verify-m034-s02-workflows.sh` with a `caller` mode that asserts the trigger set, explicit secret mapping, fork-skip condition, concurrency, and the absence of `pull_request_target`.
4. Leave a short inline explanation in the caller workflow describing why fork PRs are skipped so later maintainers do not reintroduce unsafe secret handling.

## Must-Haves

- [ ] The repo has a named authoritative verification workflow that reruns the S01 proof on trusted PR/main/manual/scheduled paths.
- [ ] Fork PRs do not receive live-publish secrets and are not upgraded to `pull_request_target` execution.
- [ ] The local verifier rejects drift in trigger policy, secret mapping, or concurrency before CI runs.

## Verification

- `bash scripts/verify-m034-s02-workflows.sh caller`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-verification.yml")'`
  - Estimate: 1.5h
  - Files: .github/workflows/authoritative-verification.yml, scripts/verify-m034-s02-workflows.sh, .github/workflows/authoritative-live-proof.yml, .github/workflows/release.yml
  - Verify: bash scripts/verify-m034-s02-workflows.sh caller
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-verification.yml")'
- [ ] **T03: Gate tag releases on the shared proof and harden workflow permissions** — The slice is only authoritative once tag releases cannot bypass the same live proof surface and the release workflow stops carrying broad write permission during ordinary builds. Rewire `release.yml` so tag releases depend on the reusable proof job, scope `contents: write` down to the actual release job, and extend the local verifier to assert the shared dependency chain. The final validation for this task is not just YAML syntax: a trusted Actions run must show `bash scripts/verify-m034-s01.sh` reaching `verify-m034-s01: ok`, and a tag release must stay blocked until that proof succeeds.

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

1. Update `.github/workflows/release.yml` so tag runs call `.github/workflows/authoritative-live-proof.yml` and `Create Release` depends on that proof plus the existing build matrices, without regressing current build/package jobs on non-tag events.
2. Move `contents: write` from workflow scope to the release job so build/proof jobs stay read-only.
3. Extend `scripts/verify-m034-s02-workflows.sh` with the full-slice assertions for shared reusable-workflow references, tag gating, and permission hardening across all three workflow files.
4. Validate the lane end to end: run the local verifier, then use a trusted `workflow_dispatch`/same-repo push and a `v*` tag run to confirm the logs show `verify-m034-s01: ok` and that `Create Release` remains downstream of the authoritative proof job.

## Must-Haves

- [ ] `release.yml` cannot create a GitHub Release for `v*` tags unless the authoritative proof has already passed.
- [ ] Workflow-wide write permissions are removed; only the actual release job keeps `contents: write`.
- [ ] The final local verifier script asserts the reusable proof, trusted-event lane, and tag-release gating as one coherent CI contract.
- [ ] Final acceptance captures real GitHub Actions evidence that the live proof runs to `verify-m034-s01: ok` on a trusted event.

## Verification

- `bash scripts/verify-m034-s02-workflows.sh`
- `ruby -e 'require "yaml"; %w[.github/workflows/authoritative-live-proof.yml .github/workflows/authoritative-verification.yml .github/workflows/release.yml].each { |f| YAML.load_file(f) }'`
- Trusted GitHub Actions evidence: a `workflow_dispatch` or same-repo `push` run shows `verify-m034-s01: ok`, and a `v*` run shows `Create Release` waiting on the authoritative proof job.
  - Estimate: 1.5h
  - Files: .github/workflows/release.yml, scripts/verify-m034-s02-workflows.sh, .github/workflows/authoritative-live-proof.yml, .github/workflows/authoritative-verification.yml
  - Verify: bash scripts/verify-m034-s02-workflows.sh
ruby -e 'require "yaml"; %w[.github/workflows/authoritative-live-proof.yml .github/workflows/authoritative-verification.yml .github/workflows/release.yml].each { |f| YAML.load_file(f) }'
