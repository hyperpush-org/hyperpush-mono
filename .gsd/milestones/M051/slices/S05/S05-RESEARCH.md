# S05 Research — Delete `reference-backend` and close the assembled acceptance rail

## Summary

This slice is mostly **path-contract, docs-truth, and deletion** work — not new runtime/language work.

The actual backend/runtime proof already moved off the repo-root app in earlier slices:

- **Mesher** owns the maintained deeper-app/runtime proof (`scripts/verify-m051-s01.sh`).
- The **retained backend fixture** owns backend-only deploy/recovery proof (`scripts/verify-m051-s02.sh`, `compiler/meshc/tests/e2e_reference_backend.rs`).
- **Tooling/editor/LSP/formatter rails** already target the retained fixture (`scripts/verify-m051-s03.sh`).

What is left for S05 is the last dependency chain around the **public proof-page verifier path**, the last **public docs mentions** of `reference-backend`, the **S02 retained-fixture compatibility assumptions**, and the **physical deletion** of `reference-backend/`.

Two concrete baseline findings:

- `bash reference-backend/scripts/verify-production-proof-surface.sh` **currently passes**. S05 should preserve that contract while relocating it; this is a move/retarget job, not a redesign job.
- `diff -qr reference-backend scripts/fixtures/backend/reference-backend` shows the repo-root copy is already **diverged** from the retained fixture (`README.md`, `jobs/worker.mpl`, `runtime/registry.mpl`, `scripts/deploy-smoke.sh`, `scripts/smoke.sh`, `scripts/stage-deploy.sh`, plus a root-only proof script and a fixture-only `tests/fixture.test.mpl`). That is a strong reason to delete the compatibility copy instead of trying to keep two authorities alive.

## Active requirements this slice serves

- **R119** — final removal of repo-root `reference-backend/` as a positive dependency while preserving the real proof through Mesher and the retained backend fixture.
- **R008** — public docs/examples must stay honest and examples-first; S05 has to remove the last public `reference-backend` wording and keep the proof-page handoff truthful.

## Skills Discovered

Already-installed skills cover the core technologies here; no `npx skills find` / installs were needed.

Relevant installed skills:

- `bash-scripting`
- `rust-testing`
- `rust-best-practices`
- `vitepress`
- `test`

Skill guidance that matters here:

- **`bash-scripting`**: keep shell verifiers in strict mode with explicit preflight checks, cleanup, and artifacted logs. The repo already uses this pattern heavily (`set -euo pipefail`, `require_file`, `require_command`, `copy_fixed_dir_or_fail`, phase markers). S05 should extend that pattern instead of inventing ad hoc shell.
- **`rust-testing`**: test behavior, not implementation details. A new `e2e_m051_s05.rs` should assert the observable contract — old path absent, new path present, delegated wrapper chain/order, retained bundle markers — not internal code structure.
- **`vitepress`**: doc edits are not done until the real VitePress build path stays green. Reuse the existing docs wrappers instead of trusting source-only text edits.

## Implementation Landscape

### 1. What still positively depends on the repo-root app

The actual runtime/tooling proof no longer depends on `reference-backend/`, but a smaller docs/verifier chain still does.

#### Public docs still leaking `reference-backend`

Only **three** public docs pages still mention it positively:

- `website/docs/docs/production-backend-proof/index.md:24,33,62`
  - still tells readers to run `bash reference-backend/scripts/verify-production-proof-surface.sh`
- `website/docs/docs/tooling/index.md:383,417`
  - still says LSP/editor proof runs against `reference-backend/`
  - still names same-file definition on `reference-backend/api/jobs.mpl`
- `website/docs/docs/distributed-proof/index.md:73`
  - still says “keep `reference-backend` as the deeper backend proof surface”

Those are the last user-visible docs leaks.

#### Wrapper scripts that will break immediately if `reference-backend/` is deleted first

These are the real blockers:

- `scripts/verify-m050-s01.sh:433,442-443`
  - `require_file` on `reference-backend/scripts/verify-production-proof-surface.sh`
  - then executes it
- `scripts/verify-m050-s03.sh:614,625-626`
  - same pattern
- `scripts/verify-m051-s02.sh`
  - still requires `reference-backend/README.md` and `reference-backend/scripts/verify-production-proof-surface.sh`
  - still reads them in contract checks
  - still copies them into the retained bundle
- `scripts/verify-m051-s04.sh`
  - does not call the old path directly, but it delegates to `verify-m050-s01.sh` and `verify-m050-s03.sh`, so it will fail transitively until those are retargeted

### 2. The clean relocation target is currently unused

There is **no existing** `scripts/verify-production-proof-surface.sh`.

That makes `scripts/verify-production-proof-surface.sh` the obvious canonical replacement path:

- top-level, public, not tied to a retired package
- consistent with the existing `scripts/verify-*.sh` naming pattern
- can preserve the exact current contract while removing the repo-root app dependency

Important gotcha when moving the file:

- current nested script computes repo root from `../..`
- after moving under `scripts/`, repo root should resolve from `..`
- the self-referenced command string inside the script also needs to change from `bash reference-backend/scripts/verify-production-proof-surface.sh` to `bash scripts/verify-production-proof-surface.sh`

### 3. S02 retained proof still preserves the compatibility copy on purpose — that now has to flip

The biggest remaining maintainer-only blocker is S02’s retained-fixture contract.

Files:

- `scripts/fixtures/backend/reference-backend/README.md`
- `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl`
- `compiler/meshc/tests/e2e_m051_s02.rs`
- `scripts/verify-m051-s02.sh`

Current state:

- retained README still says later slices must not delete the compatibility path yet
- retained fixture test still expects the README to mention `reference-backend/README.md`
- S02 Rust contract still expects bundle/readme/verifier markers for the repo-root compatibility files
- S02 assembled verifier still checks, copies, and bundle-validates those files

This is now exactly backwards for S05. The retained fixture should become authoritative **without** expecting the repo-root copy to exist.

One small internal truth fix is easy to forget:

- `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql:1`
  - comment still says `-- Derived from reference-backend/migrations/...`
  - after deletion, that comment becomes false and should point at the retained fixture path or just `migrations/...`

### 4. Existing public-doc contracts have a real coverage gap

The remaining stale public wording is surviving because existing tests are too narrow in two places.

#### Tooling page gap

`website/docs/docs/tooling/index.md` still leaks `reference-backend/`, but the current contracts only ban the old runbook path and some old commands.

Relevant test file:

- `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs:64-69`

`directRepoRootBackendMarkers` currently bans:

- `reference-backend/README.md`
- `meshc test reference-backend`
- `meshc test reference-backend/tests`
- `meshc test reference-backend/tests/config.test.mpl`
- `meshc fmt --check reference-backend`

It does **not** ban:

- `reference-backend/`
- `reference-backend/api/jobs.mpl`

That is why the tooling page leak survived.

Best existing place to fail-close this in the long term:

- `scripts/tests/verify-m036-s03-contract.test.mjs`

Reason:

- it already owns the public tooling page + editor README contract
- it already enforces the generic VS Code README wording:
  - `real stdio JSON-RPC against a small backend-shaped Mesh project`
  - `same-file go-to-definition inside backend-shaped project code`
- `verify-m050-s02.sh` already delegates to it

The easiest/cleanest docs rewrite is to copy that already-shipped generic wording from `tools/editors/vscode-mesh/README.md` into the website tooling page.

#### Distributed-proof gap

`website/docs/docs/distributed-proof/index.md:73` still has a stale bare-word bullet about `reference-backend`, but the secondary-surface contract only bans:

- stale runbook link (`reference-backend/README.md`)
- retained fixture path (`scripts/fixtures/backend/reference-backend/`)

Relevant test file:

- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`

So the bare-word bullet survives today even though the public story has already moved on.

### 5. What is already safe and should not be reopened

These areas are already cut over and do **not** need structural rework for S05:

- `compiler/meshc/tests/e2e_reference_backend.rs`
  - already runs the retained backend fixture
- `compiler/meshc/tests/e2e_lsp.rs`, `tooling_e2e.rs`, `compiler/mesh-lsp/src/analysis.rs`, `compiler/mesh-fmt/src/lib.rs`
  - already point at the retained fixture
- editor-host smoke sources
  - already use retained fixture paths

In other words: the risky part is **contracts + docs + deletion order**, not compiler/runtime behavior.

Also safe to leave alone unless you want extra cleanup:

- negative mutation tests that mention `reference-backend` only as stale text to reject
- legacy-path sentinel helpers like `LEGACY_COMPAT_ROOT_RELATIVE` / `legacy_repo_root_binary_path()` that only compare paths and do not require the directory to exist

## Recommendation

### Recommended execution order

1. **Create `scripts/verify-production-proof-surface.sh` and retarget all positive callers first.**
   - Do this before deleting `reference-backend/`.
   - Preserve the current proof contract; do not widen/narrow scope during the move.

2. **Fix the last three public docs leaks and strengthen the closest existing contracts.**
   - `production-backend-proof/index.md` → new verifier path, no “compatibility verifier” framing
   - `tooling/index.md` → generic backend-shaped wording, no repo-root package/path examples
   - `distributed-proof/index.md` → replace the stale “keep `reference-backend`…” bullet with post-deletion truth

3. **Flip S02 retained-fixture materials from “compatibility copy still preserved” to post-deletion truth.**
   - README
   - fixture test
   - S02 Rust contract
   - S02 shell verifier
   - deploy SQL comment

4. **Delete `reference-backend/` and remove `.gitignore:26`.**
   - no stub
   - no symlink
   - no partial directory preservation

5. **Add the final slice-owned S05 contract and assembled verifier.**
   - assert old tree absent
   - assert new public proof script path present
   - delegate to S01-S04 wrappers
   - retain one final bundle

## Natural task seams

### T1 — Relocate the public proof-page verifier and retarget positive callers

Primary files:

- new: `scripts/verify-production-proof-surface.sh`
- update: `website/docs/docs/production-backend-proof/index.md`
- update: `scripts/verify-m050-s01.sh`
- update: `scripts/verify-m050-s03.sh`
- update: `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
- update: `scripts/verify-m051-s04.sh`
- update: `compiler/meshc/tests/e2e_m050_s01.rs`
- update: `compiler/meshc/tests/e2e_m050_s03.rs`
- update: `compiler/meshc/tests/e2e_m051_s04.rs`

Notes:

- keep phase name `production-proof-surface` stable in the existing wrappers
- update artifact hint strings / `require_file` paths too, not just command text
- `e2e_m050_s01.rs` / `e2e_m050_s03.rs` are not in the active wrapper chain today, but they hardcode the old path and are worth fixing while touching the same surface

### T2 — Remove the last public `reference-backend` wording and close the contract gaps

Primary files:

- `website/docs/docs/tooling/index.md`
- `website/docs/docs/distributed-proof/index.md`
- `scripts/tests/verify-m036-s03-contract.test.mjs`
- `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`
- `scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`
- `compiler/meshc/tests/e2e_m051_s04.rs`
- `scripts/verify-m051-s04.sh`

Optional but reasonable if you want historical rails to own the wording directly:

- `compiler/meshc/tests/e2e_m047_s06.rs`

Notes:

- reuse the already-good generic wording from `tools/editors/vscode-mesh/README.md`
- this is a docs-truth task, not a tooling implementation task
- current contracts do not catch the existing stale tooling/distributed-proof language; add explicit exclusions for those markers

### T3 — Retire S02 compatibility-boundary assumptions and delete the tree

Primary files:

- `scripts/fixtures/backend/reference-backend/README.md`
- `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl`
- `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql`
- `compiler/meshc/tests/e2e_m051_s02.rs`
- `scripts/verify-m051-s02.sh`
- `.gitignore`
- remove tracked tree: `reference-backend/`

Notes:

- this task should happen after T1/T2, not before
- S02 verifier is the one active shell surface still preserving the repo-root copy in bundle-shape assertions

### T4 — Add final S05 contract + assembled verifier

New files expected:

- `compiler/meshc/tests/e2e_m051_s05.rs`
- `scripts/verify-m051-s05.sh`

Recommended shape:

- cheap fail-closed contract first
- then delegated wrappers in order:
  - `bash scripts/verify-m051-s01.sh`
  - `bash scripts/verify-m051-s02.sh`
  - `bash scripts/verify-m051-s03.sh`
  - `bash scripts/verify-m051-s04.sh`
- then copy:
  - `.tmp/m051-s01/verify`
  - `.tmp/m051-s02/verify`
  - `.tmp/m051-s03/verify`
  - `.tmp/m051-s04/verify`
- final bundle should keep the established M051 verifier schema:
  - `status.txt`
  - `current-phase.txt`
  - `phase-report.txt`
  - `full-contract.log`
  - `latest-proof-bundle.txt`

## Verification

### Cheap / direct checks

- `bash scripts/verify-production-proof-surface.sh`
- `test ! -e reference-backend`
- `cargo test -p meshc --test e2e_m051_s05 -- --nocapture`

### Existing subsystem-authoritative replays that should stay green post-deletion

- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s01.sh`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s02.sh`
- `bash scripts/verify-m051-s03.sh`
- `bash scripts/verify-m051-s04.sh`

### Final assembled rail

- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s05.sh`

## Risks / gotchas

- **Deletion order matters.** If `reference-backend/` is deleted before the proof-page verifier is relocated, `verify-m050-s01.sh`, `verify-m050-s03.sh`, `verify-m051-s02.sh`, and therefore `verify-m051-s04.sh` all fail immediately.
- **The current docs contracts are too weak.** If you only delete the tree and patch the old proof-script path, stale public wording can still survive in `tooling/index.md` and `distributed-proof/index.md`.
- **Do not keep a stub or symlink** at `reference-backend/`. That contradicts the slice demo (“repo ships without `reference-backend/`”).
- **Preflight `DATABASE_URL` once** in the final S05 verifier. S01 and S02 both need it, and failing early is much clearer than letting child bundles fail deep in delegated logs.
- **Dormant historical Rust contract tests** (`e2e_m050_s01.rs`, `e2e_m050_s03.rs`) still hardcode the old proof-script path. They are not the current wrapper entrypoints, but leaving them stale will surprise anyone who runs those tests directly.
