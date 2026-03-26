---
estimated_steps: 4
estimated_files: 2
skills_used:
  - github-workflows
---

# T01: Create the reusable live-proof workflow and local contract verifier

**Slice:** S02 — Authoritative CI verification lane
**Milestone:** M034

## Description

Land the single GitHub Actions unit that knows how to run the real Mesh package-manager proof. Reuse the S01 verifier instead of translating its publish/install checks into YAML. The workflow should run only as a reusable `workflow_call` unit, on Linux x86_64, with the same Rust and LLVM 21 prerequisites the Linux release build already needs. When the proof fails, keep `.tmp/m034-s01/verify/**` available as an artifact so later tasks and future operators can inspect the exact broken phase.

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

1. Add `.github/workflows/authoritative-live-proof.yml` as a reusable `workflow_call` workflow with one stable job name, Ubuntu Linux runner, Rust toolchain setup, and the LLVM 21 bootstrap copied from the existing Linux release path.
2. Wire explicit `MESH_PUBLISH_OWNER` and `MESH_PUBLISH_TOKEN` secrets into that reusable workflow and run `bash scripts/verify-m034-s01.sh` unchanged.
3. Upload `.tmp/m034-s01/verify/**` when the proof fails so the exact verifier phase logs remain available in Actions artifacts.
4. Add `scripts/verify-m034-s02-workflows.sh` with a `reusable` mode that mechanically asserts the reusable workflow calls the S01 verifier, configures the Linux toolchain, and retains failure artifacts.

## Must-Haves

- [ ] `.github/workflows/authoritative-live-proof.yml` is the only GitHub Actions definition that knows how to run the live Mesh package-manager proof.
- [ ] The reusable workflow reuses `scripts/verify-m034-s01.sh` unchanged and retains `.tmp/m034-s01/verify/**` for failure diagnosis.
- [ ] A local verifier script can mechanically reject drift in the reusable workflow contract before CI runs.

## Verification

- `bash scripts/verify-m034-s02-workflows.sh reusable`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/authoritative-live-proof.yml")'`

## Observability Impact

- Signals added/changed: a stable reusable proof job plus uploaded `.tmp/m034-s01/verify/**` artifacts on failure.
- How a future agent inspects this: open the reusable workflow logs/artifacts or run `bash scripts/verify-m034-s02-workflows.sh reusable` locally.
- Failure state exposed: whether drift came from toolchain bootstrap, secret wiring, or the underlying S01 proof.

## Inputs

- `scripts/verify-m034-s01.sh` — canonical live publish/install verifier that must remain the single proof surface.
- `.github/workflows/release.yml` — source of the existing Linux LLVM/Rust setup pattern to reuse for the proof runner.

## Expected Output

- `.github/workflows/authoritative-live-proof.yml` — reusable workflow that runs the live proof on Linux with explicit secret inputs.
- `scripts/verify-m034-s02-workflows.sh` — local contract verifier with a `reusable` mode for the new workflow.
