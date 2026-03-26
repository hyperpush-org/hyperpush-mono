# M034 / S02 — Research

**Date:** 2026-03-26

## Summary

- Direct requirement pressure is on **R045** and **R046** from the milestone context: CI/release flows must prove shipped surfaces, and the package-manager path must be exercised end to end on the real path. **R047** is only lightly supported here by establishing a reusable verification pattern that S04 can consume later.
- The repo already has the right proof primitive: **`scripts/verify-m034-s01.sh`** is the authoritative live registry verifier. S02 should **orchestrate that command in GitHub Actions**, not build a second publish/install proof in YAML.
- Current workflows are still **artifact/deploy oriented**:
  - `release.yml` builds cross-platform artifacts and creates GitHub Releases.
  - `deploy-services.yml` deploys Fly apps and curls basic URLs.
  - `deploy.yml` builds/deploys docs.
  - `publish-extension.yml` compiles/packages/publishes the VS Code extension.
  - None of them currently run `bash scripts/verify-m034-s01.sh`, any `scripts/verify-*`, or any `cargo test` target.
- The main S02 constraint is **GitHub Actions secret and trust boundaries**, not Mesh code. The S01 verifier needs pre-provisioned `MESH_PUBLISH_OWNER` / `MESH_PUBLISH_TOKEN`, and GitHub does **not** pass normal secrets to fork-triggered `pull_request` runs. Using `pull_request_target` to run checked-out PR code with those secrets would be the wrong trade.
- The second real constraint is **toolchain setup**: `scripts/verify-m034-s01.sh` builds `meshpkg` and `meshc` itself (`cargo build -q -p meshpkg -p meshc`), so a CI job invoking it on Linux must also install Rust + LLVM 21 similarly to the Linux path already embedded in `release.yml`.

## Requirements Focus

The compact `REQUIREMENTS.md` excerpt in context was truncated before the R045-R047 entries, but the milestone context names the relevant slice requirements explicitly.

### R045 — CI/CD and release flows prove shipped Mesh surfaces

S02 is the first slice that can make this requirement mechanically true. Right now release/deploy workflows only prove that assets can be built, uploaded, or deployed; they do **not** prove the real package-manager contract.

### R046 — package manager tested end to end on the real path

S01 already produced the canonical proof surface (`scripts/verify-m034-s01.sh`). S02’s job is to promote that proof from an on-demand local command into an **authoritative CI lane** that reruns it on trusted PR/release paths.

### R047 — extension/editor release hardening support only

S02 should not try to solve extension truth itself; S04 owns that. What S02 can provide is a reusable CI pattern: thin workflow orchestration around canonical verifier scripts, explicit secret handling, and release gating based on real proof instead of packaging success.

## Skills Discovered

- Already installed and directly relevant:
  - **`github-workflows`**
    - Key rule applied here: **“No errors is not validation. Prove observable change.”**
    - Key planning implication: read current GitHub Actions docs before deciding on `workflow_call`, secret passing, and concurrency behavior.
- Searched for missing directly relevant skills:
  - `Fly.io` → `thinkfleetai/thinkfleet-engine@flyio-cli-public` (14 installs)
- No new skill installed. Fly is adjacent to the deploy workflow, but S02 is primarily about GitHub Actions orchestration and reuse of the existing Mesh verifier.

**Important note:** the `github-workflows` skill assumes a helper script `scripts/ci_monitor.cjs`, but this repo does **not** contain that file. Do not plan around it unless a later task explicitly adds it.

## Recommendation

### Recommended architecture

**Keep responsibilities split across workflows, but factor the proof into one reusable GitHub Actions workflow that shells out to `bash scripts/verify-m034-s01.sh`.**

That means:

1. **New reusable workflow** for the authoritative live proof
   - owns Linux toolchain setup, secret wiring, and the single call to `bash scripts/verify-m034-s01.sh`
   - exposes explicit `workflow_call` inputs/secrets
   - emits one stable check/job name future slices can depend on

2. **A PR/main/manual/scheduled caller workflow**
   - runs the reusable proof on trusted events
   - gives the repo a named CI lane independent from asset packaging
   - optionally adds `schedule` to close the S01 “monitoring gap” called out in the summary

3. **`release.yml` calls the same reusable proof before `Create Release`**
   - so tag releases cannot go green on artifact builds alone
   - preserves the current release workflow’s role as the artifact publisher

This keeps the milestone’s separate responsibilities intact:
- release workflow still publishes release assets
- deploy workflows still deploy services/docs
- extension workflow still publishes the extension
- the proof surface is shared, not copy-pasted

### Strong recommendations

- **Do not hand-roll a second publish/install verifier in YAML.** The YAML should orchestrate; `scripts/verify-m034-s01.sh` should continue owning the contract.
- **Do not use `pull_request_target`** to run checked-out PR code with publish secrets. That is the wrong security model for a script that builds and runs repository code.
- **Do not fan the live proof across the existing cross-platform matrix.** The packaging matrix already proves asset builds. The authoritative package-manager truth only needs **one Linux x86_64 job**.
- **Preserve the verifier’s secret handling model.** The script already logs in via stdin (`run_command_with_stdin ... meshpkg --json login`), which is better for CI than putting the token on the process command line.

### Safest event policy

The repo needs an explicit answer for fork PRs. Based on GitHub’s current secret model, the safest default is:

- run the live proof on:
  - `push` to `main`
  - tag pushes (`v*`)
  - `workflow_dispatch`
  - `schedule`
  - **same-repo** `pull_request` events only
- skip the live proof on **fork PRs**
- keep secret-free artifact/build checks on ordinary `pull_request`

If the user later wants fork PRs to run live proof before merge, that is a separate security design problem and should not be quietly solved with `pull_request_target`.

## Don’t Hand-Roll

- Don’t reimplement the S01 phases as many inline GH Actions shell steps.
- Don’t duplicate registry HTTP assertions in both a workflow file and `scripts/verify-m034-s01.sh`.
- Don’t merge `release.yml`, `deploy-services.yml`, and `deploy.yml` into one mega-workflow just to get a dependency chain. The milestone context explicitly says those responsibilities are already split and should stay distinct.
- Don’t switch the workflow to `meshpkg login --token ...` just because the website docs show that UX path; for CI, the existing verifier’s stdin login is the safer contract.

## Implementation Landscape

### `.github/workflows/release.yml`

Current role:
- triggers on `push` to `main`, tag pushes `v*`, and all `pull_request` events (`.github/workflows/release.yml:3-7`)
- grants **workflow-wide** `contents: write` (`.github/workflows/release.yml:9-10`)
- runs two build matrices:
  - `build` for `meshc` (`:13`)
  - `build-meshpkg` for `meshpkg` (`:219`)
- creates a GitHub Release only on tag refs (`:278-297`)

What is missing:
- no `cargo test`
- no `bash scripts/verify-*`
- no package-manager proof
- no dependency on deploy workflows

Natural S02 seam:
- add an **authoritative proof job** (or reusable-workflow call) that must pass before `release`
- if touching permissions, narrow `contents: write` to the release job instead of leaving it workflow-wide

### `.github/workflows/deploy-services.yml`

Current role:
- tag/manual only (`.github/workflows/deploy-services.yml:3-6`)
- deploys registry and packages website to Fly (`:13-40`)
- runs only basic post-deploy curls against:
  - `https://api.packages.meshlang.dev/api/v1/packages`
  - `https://packages.meshlang.dev`
  - `https://meshlang.dev` (`:45-65`)

Planner implication:
- keep this workflow separate in S02
- later slices can compose it into the full public release story, but S02 should not replace its responsibility with package-manager proof or vice versa

### `.github/workflows/deploy.yml`

Current role:
- builds/deploys docs on `main`, `v*`, and manual dispatch (`.github/workflows/deploy.yml:1-59`)
- runs `npm ci` + `npm run build` for `website/`

Planner implication:
- S02 does not need to fuse docs deployment into the release lane
- this remains a downstream proof surface for S05

### `.github/workflows/publish-extension.yml`

Current role:
- tag-only extension publish workflow
- `npm ci`, `npm run compile`, `npx vsce package --no-dependencies`, then publish (`.github/workflows/publish-extension.yml:25-50`)
- Open VSX publish is still `continue-on-error: true` (`:40-42`)

Planner implication:
- S02 should not harden this directly
- but the reusable proof-lane pattern from S02 is a good template for S04

### `scripts/verify-m034-s01.sh`

This is the central proof surface and should remain so.

Important contract points:
- requires `MESH_PUBLISH_OWNER` / `MESH_PUBLISH_TOKEN` (`scripts/verify-m034-s01.sh:325-334`)
- optional `MESH_PROOF_VERSION` override exists, but auto-generation is already good enough for CI (`:338-347`)
- builds local tooling itself (`:427`)
- logs in via stdin, not CLI arg (`:431`)
- publishes live package (`:443`)
- verifies install path and `mesh.lock` (`:481-527`)
- verifies named install leaves manifest unchanged and reports `manifest_changed: false` (`:555-584`, `compiler/meshpkg/src/install.rs:29-30`, `:324-333`)
- verifies duplicate publish rejection (`scripts/verify-m034-s01.sh:615`)
- verifies exact package detail/search page visibility (`:622-623`)
- succeeds only on final `verify-m034-s01: ok` (`:626`)

Planner implications:
- CI should pass the existing env names straight through
- CI should archive or at least preserve the script’s `.tmp/m034-s01/verify/<version>/` logs on failure if possible
- CI does **not** need to manufacture a proof version unless traceability demands it later

### Token source / secret provisioning

The token is not self-mintable in CI.

Source of truth:
- packages website publish page points users to GitHub sign-in and says a publish token is auto-generated (`packages-website/src/routes/publish/+page.svelte:4,29`)
- token page shows the one-time token and the `meshpkg login --token ...` UX (`packages-website/src/routes/token/+page.svelte:27,94`)
- registry auth callback auto-creates the token and redirects the frontend with `value=<token>&login=<login>` (`registry/src/routes/auth.rs:93-99`)
- raw tokens are only shown at creation time; DB stores the hash (`registry/src/routes/auth.rs:201-208`)

Planner implication:
- S02 must consume **pre-provisioned GitHub Actions secrets** for `MESH_PUBLISH_OWNER` and `MESH_PUBLISH_TOKEN`
- there is no honest “mint one inside the workflow” path in-tree

### Toolchain setup seam

The verifier’s `cargo build -q -p meshpkg -p meshc` means the live proof job needs the Mesh compiler build prerequisites.

Relevant existing setup is only in `release.yml`:
- installs LLVM 21 tarballs or brew packages (`.github/workflows/release.yml:57-117`)
- exports `LLVM_SYS_211_PREFIX` (`:63,112,117,163,172,178`)

Planner implication:
- either duplicate the **Linux x86_64 subset** into the new proof job
- or extract a shared setup primitive if the diff stays understandable
- there is currently **no** `.github/actions/*` composite action to reuse

## Findings

### 1. Current GitHub Actions truth stops at build/package/deploy, not at proof

A workspace-wide workflow grep found no current use of:
- `cargo test`
- `meshc test`
- `bash scripts/verify-*`
- shell verifier syntax checks

The only builds in workflows today are asset builds and docs/extension compilation. That is the exact gap S02 exists to close.

### 2. The release story is split across workflows with no proof chain tying them together

Today’s green surfaces are separate:
- artifacts: `release.yml`
- registry/packages website deploy: `deploy-services.yml`
- docs deploy: `deploy.yml`
- extension publish: `publish-extension.yml`

That matches the milestone context: responsibilities are already split and should stay split. S02 should share **proof**, not collapse these workflows into one.

### 3. The S01 verifier is already CI-friendly enough to be called directly

The script already has the right CI shape:
- deterministic env contract
- JSON-based CLI checks
- direct HTTP checks
- first-failing-phase logging
- unique per-run version generation
- no secret echoing

That means S02 is mostly workflow composition plus secret/toolchain setup.

### 4. GitHub’s secret model is the main PR-design constraint

Current GitHub docs say:
- non-`GITHUB_TOKEN` secrets are **not passed** to workflows triggered from forks
- reusable workflows do **not** automatically receive secrets
- `pull_request_target` runs in the base repo context and is the dangerous choice if the workflow checks out and runs PR code

Planner implication:
- the honest safe design is not “run the live publish proof on every fork PR”
- same-repo PRs are fine; forks need either skip/manual-post-merge handling or a different security posture the user explicitly accepts

### 5. Reusable workflows fit this slice well, but have one caveat

GitHub’s reusable workflow docs match the natural S02 structure:
- define `on.workflow_call.inputs` / `on.workflow_call.secrets`
- call them from jobs with `uses:`
- pass named secrets explicitly or via `inherit`

Caveat:
- environment secrets cannot be passed from the caller via `workflow_call`
- if the planner wants environment protection, the reusable workflow job itself must own that environment, or the repo should use ordinary repository/org secrets instead

### 6. The live proof should be one Linux job, not another matrix

The existing release workflow already has 10 build jobs across `meshc` + `meshpkg`. Repeating the real registry publish/install proof across that matrix would add cost without meaningfully increasing truth.

The authoritative proof only needs one trusted host that can:
- build host `meshc` / `meshpkg`
- log into the real registry
- publish/install/download/check
- reach the public registry and packages site

That strongly points to a single Ubuntu x86_64 verifier job.

### 7. Permission hardening is a cheap side benefit if `release.yml` is edited

`release.yml` currently grants `contents: write` workflow-wide even though only the tag-release job appears to need it. If S02 edits that workflow anyway, it is a good moment to scope permissions down so the future proof job does not inherit unnecessary write capability.

### 8. Scheduling is a natural part of S02, not extra gold-plating

S01’s summary explicitly calls out a monitoring gap: there is no scheduled or always-on rerun of the live proof. Adding a lightweight scheduled caller around the canonical verifier fits the slice goal and uses the same proof surface the release lane needs anyway.

## Natural Seams / Taskable Units

1. **Reusable authoritative proof workflow**
   - likely new file under `.github/workflows/`
   - owns Linux runner setup, secret inputs, concurrency, and the single `bash scripts/verify-m034-s01.sh` call
   - should be the only workflow-level place that knows how to run the live proof

2. **PR/main/manual/scheduled verification caller**
   - likely a new top-level workflow file
   - triggers on `pull_request`, `push` to `main`, `workflow_dispatch`, and likely `schedule`
   - uses event filters/conditions so live proof only runs on trusted events

3. **Tag release gating in `release.yml`**
   - make release creation depend on the same reusable proof workflow
   - keep artifact packaging intact; just prevent tag releases from bypassing the live proof

4. **Optional toolchain-sharing cleanup**
   - only if duplication becomes ugly
   - could be a small helper/composite action, but this is optional for the first honest implementation

5. **Observability/retention polish**
   - if feasible, upload `.tmp/m034-s01/verify/**` as a workflow artifact on failure
   - not required to make the lane truthful, but very useful for post-failure debugging

## What to Build / Prove First

1. **Build the reusable live-proof job first.**
   - fastest way to prove the workflow can actually run `bash scripts/verify-m034-s01.sh` with LLVM + secrets on a GitHub runner

2. **Then gate `release.yml` on it.**
   - once the live-proof job is real, block tag releases behind it

3. **Then add the separate CI caller for PR/main/schedule/manual dispatch.**
   - this gives the repo a named verification lane beyond release tags

4. **Only then optimize.**
   - if runtime or duplication is painful, consider a shared setup action or optional env overrides for prebuilt binaries
   - do not start with abstraction before the live proof has been shown to run in Actions at all

## Verification Targets

### Local preflight

These are cheap checks the executor can run before pushing workflow changes:

```bash
bash -n scripts/verify-m034-s01.sh
rg -n "verify-m034-s01\.sh|workflow_call|schedule|workflow_dispatch|MESH_PUBLISH_OWNER|MESH_PUBLISH_TOKEN|concurrency" .github/workflows -S
```

Notes:
- there is no repo-local `actionlint` setup today
- static grep/syntax is only preflight; it is **not** acceptance

### Acceptance proof for S02

Per the `github-workflows` skill, the workflow is only valid once it proves an observable change. For S02 that means real Actions evidence:

1. **Trusted PR or manual/main-branch run**
   - the new verification workflow shows the authoritative proof job running
   - that job executes `bash scripts/verify-m034-s01.sh`
   - the log ends with `verify-m034-s01: ok`

2. **Tag release run**
   - `release.yml` does **not** create the GitHub Release unless the authoritative proof job passes first
   - the proof job is visibly part of the tag run dependency chain

3. **If schedule is added**
   - a scheduled run can execute the same proof job without code changes, proving the lane is also a drift monitor and not just a release hook

### Still-authoritative underlying command

The canonical proof remains:

```bash
bash scripts/verify-m034-s01.sh
```

S02 is successful when GitHub Actions reruns that same proof surface in the right places rather than replacing it with weaker approximations.

## Risks / Open Questions

- **Fork PR policy:** is same-repo PR + main/tag/manual/schedule enough, or does the user want a maintainer-approved fork-PR live proof lane despite the security tradeoffs?
- **Secret storage model:** repository secrets are simplest; environment secrets add approval controls but interact awkwardly with reusable workflows.
- **Permission tightening scope:** if `release.yml` is touched, should S02 also narrow `contents: write` to the release job only?
- **Toolchain duplication:** is one copied Linux LLVM setup block acceptable for now, or does the repo want shared workflow setup immediately?
- **Artifact retention:** should S02 upload `.tmp/m034-s01/verify/**` on failure, or leave that as nice-to-have once the lane itself is real?

## Sources

External docs consulted because S02 depends on current GitHub Actions behavior:

- GitHub Docs — **Using secrets in GitHub Actions**
  - https://docs.github.com/actions/security-guides/using-secrets-in-github-actions
  - Key facts used: non-`GITHUB_TOKEN` secrets are not passed to fork-triggered workflows; secrets are not automatically passed to reusable workflows.

- GitHub Docs — **Reuse workflows**
  - https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows
  - Key facts used: `workflow_call` supports explicit inputs/secrets; callers pass secrets explicitly or via `inherit`; environment secrets are not passed from the caller.

- GitHub Docs — **Control the concurrency of workflows and jobs**
  - https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency
  - Key fact used: concurrency groups can use workflow/ref expressions and fallbacks like `github.head_ref || github.run_id`.

- GitHub Docs — **Managing GitHub Actions settings for a repository**
  - https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository
  - Key fact used: `pull_request_target` workflows run in base-branch context and are not protected by the same fork-approval behavior as ordinary fork PR workflows.
