# S05 Research — Assembled contract proof and minimal public touchpoints

## Summary

This is **targeted closeout work**, not new product architecture.

The repo already has the underlying proof rails for M048:
- S01 gave the authoritative override-entry build/test acceptance rail.
- S02 gave the live LSP/editor/package rails.
- S03 gave the installer-backed self-update rails.
- S04 gave the syntax + skill drift rails.

What is still missing for S05 is:
1. **one assembled verifier** that replays the retained rails together, and
2. **a very small public-truth pass** so the public docs/READMEs stop omitting or overstating these surfaces.

The main stale public gaps today are:
- `README.md` does **not** mention installer-backed `meshc update` / `meshpkg update`.
- `README.md` and `website/docs/docs/tooling/index.md` do **not** mention optional `[package].entrypoint` even though default `main.mpl` remains correct.
- `website/docs/docs/tooling/index.md` still documents `meshpkg publish` generically, but does not mention the shipped nested-source archive behavior for override-entry packages.
- `tools/editors/vscode-mesh/README.md` omits the shipped `@cluster` / `@cluster(N)` grammar truth and override-entry proof, and it still overclaims **"jump to definitions across files"** even though the truthful override-entry proof currently uses hover, not cross-file definition.
- There is **no single repo command** that ties S01 + S02 + S03 + S04 together.

## Requirements in Scope

- **R112** — still the main closeout target. S01/S02 advanced it, but S05 is the natural place to validate the end-to-end default-plus-override contract across build/test/LSP/editor/package surfaces.
- **R113** — already has validation evidence from S03, but S05 should add the missing public-touchpoint truth for `meshc update` / `meshpkg update` and replay the retained rail in the assembled verifier.
- **R114** — already has validation evidence from S04, but S05 should keep the public editor touchpoints truthful and replay the retained grammar/skill rails in the assembled verifier.

Bookkeeping note: S03/S04 summaries both recorded that `gsd_requirement_update` could not resolve `R113` / `R114` in this environment even though the rendered file and decisions already carry the proof. Do not let that DB issue get mixed into the implementation unless milestone closeout explicitly needs it.

## Skills Discovered

No new external skills were needed; all core technologies already have installed skills.

Relevant installed skills:
- **`vscode-extension-expert`** — useful rule: public extension claims should track the real Extension Development Host smoke and activation surface, not broader hypothetical capability. This directly supports fixing the VS Code README wording.
- **`neovim`** — useful rule: keep repo-owned Neovim guidance scoped to the actual native `vim.lsp` / Neovim 0.11 path instead of expanding into generic plugin-manager claims. This supports leaving the Neovim README mostly untouched.
- **`rust-best-practices`** — useful rule: keep acceptance/contract tests small and descriptive, with focused helpers instead of a monolithic harness. This matters if S05 adds a Rust contract test.

## Recommendation

Use the **lightest sufficient closeout pattern**:
- add **one dedicated S05 docs/verifier contract test**,
- add **one assembled shell verifier**,
- update only the **minimum public touchpoints** that are actually stale.

Recommended file scope:
- `scripts/verify-m048-s05.sh` — new assembled verifier
- `scripts/tests/verify-m048-s05-contract.test.mjs` — new fast drift test for public touchpoints + verifier contract
- `README.md` — small install/entrypoint/verifier truth updates
- `website/docs/docs/tooling/index.md` — the main public tooling truth page; add update/entrypoint/publish/editor truth here
- `tools/editors/vscode-mesh/README.md` — fix the stale VS Code-specific claims

Keep these surfaces **out of scope unless a real failure appears**:
- `tools/editors/neovim-mesh/README.md` — already truthful for manifest-first roots and `@cluster`
- `tools/skill/mesh/**/*` — already covered by the S04 skill contract rail
- `website/docs/docs/getting-started/index.md` — default `main.mpl` onboarding is still truthful because the contract is default-plus-override, not override-only

## Implementation Landscape

### Existing proof rails to reuse

- `compiler/meshc/tests/e2e_m048_s01.rs`
  - **Authoritative S01 replay point** for override-entry build/test behavior.
  - S01 summary explicitly says later closeout work should keep this rail as the replay point.
  - Covers default, override-precedence, override-only, and override-entry test-discovery scenarios.

- `scripts/verify-m036-s02.sh`
  - Existing Neovim wrapper with separate `syntax`, `lsp`, `neovim`, and `all` phases.
  - `lsp` phase replays `cargo test -q -p meshc --test e2e_lsp -- --nocapture` and then the Neovim LSP smoke.
  - `syntax` phase replays corpus materialization and Neovim syntax smoke.

- `tools/editors/vscode-mesh/src/test/suite/extension.test.ts`
  - Already materializes the override-entry fixture:
    - `mesh.toml`
    - `entrypoint = "lib/start.mpl"`
    - `lib/start.mpl`
    - `lib/support/message.mpl`
  - The truthful override-entry proof here is:
    - clean diagnostics on entry/support files
    - hover on the imported nested helper
  - Same-file definition proof is still on `reference-backend/api/jobs.mpl`; the override-entry path is **not** proving cross-file definition.

- `compiler/meshpkg/src/publish.rs`
  - Already has the S02 archive tests proving:
    - nested override-entry support modules are archived
    - hidden paths and `*.test.mpl` are excluded
    - root `main.mpl` is preserved when both root and override entries exist

- `compiler/mesh-pkg/tests/toolchain_update.rs`
- `compiler/meshc/tests/tooling_e2e.rs`
- `compiler/meshpkg/tests/update_cli.rs`
- `compiler/meshc/tests/e2e_m048_s03.rs`
  - Together these are the retained S03 update rails.

- `scripts/verify-m036-s01.sh`
- `scripts/tests/verify-m036-s02-contract.test.mjs`
- `scripts/tests/verify-m048-s04-skill-contract.test.mjs`
  - These are the retained S04 grammar + Neovim contract + skill-bundle rails.

### Existing closeout templates

- `scripts/verify-m047-s05.sh`
  - Good shell template for a milestone closeout verifier with:
    - `status.txt`
    - `current-phase.txt`
    - `phase-report.txt`
    - `full-contract.log`
    - optional retained bundle copying

- `scripts/verify-m047-s06.sh`
  - Heavier shell template with docs contract guards + retained bundle shape checks.

- `compiler/meshc/tests/e2e_m047_s05.rs`
- `compiler/meshc/tests/e2e_m047_s06.rs`
  - Good templates **if** the planner wants a Rust contract test for docs/verifier drift.

- `scripts/tests/verify-m036-s03-contract.test.mjs`
- `scripts/tests/verify-m048-s04-skill-contract.test.mjs`
  - Good templates **if** the planner wants the lighter Node-based contract-test route.

### Public touchpoints that are actually stale

- `README.md`
  - Still says `meshc init` creates `main.mpl`, which is fine for the default path.
  - Missing the new optional override-entry truth.
  - Missing installer-backed update commands.
  - Missing any pointer to the new assembled M048 verifier.

- `website/docs/docs/tooling/index.md`
  - Best place to add the public tooling truth:
    - installer-backed toolchain update commands
    - optional `[package].entrypoint`
    - recursive publish archive behavior for override-entry packages
    - a top-level pointer to the new assembled verifier
  - Also the best place to add the VS Code/LSP wording about manifest-first override-entry proof and `@cluster` decorator syntax.

- `tools/editors/vscode-mesh/README.md`
  - Currently stale in two ways:
    1. it omits the shipped `@cluster` / `@cluster(N)` grammar truth and the override-entry fixture proof
    2. it still says **"Verified Go to Definition -- jump to definitions across files"**, which is broader than the current truthful proof surface

## Suggested Task Decomposition

### Task 1 — Public truth pass

Files:
- `README.md`
- `website/docs/docs/tooling/index.md`
- `tools/editors/vscode-mesh/README.md`

What to add/change:
- **README**
  - After install verification, add a short note that installer-backed toolchains can be refreshed with `meshc update` or `meshpkg update`.
  - After the `meshc init` hello-world scaffold note, add one short note that `main.mpl` is still the default executable entrypoint, but executable packages can set `[package].entrypoint = "lib/start.mpl"` when they want a non-root entry file.
  - Add a short pointer to the new assembled verifier `bash scripts/verify-m048-s05.sh`.

- **Tooling page**
  - Add an explicit **toolchain update** subsection that says these commands refresh the installed `meshc` + `meshpkg` pair through the canonical installer path; they do **not** mean project dependency update.
  - In the `mesh.toml` / project section, add the optional `[package].entrypoint` example and keep `main.mpl` as the default.
  - In the `meshpkg publish` section, add one sentence about preserving nested project-root-relative `.mpl` paths while excluding hidden and test-only files.
  - In the LSP / VS Code section, add the manifest-first override-entry proof note and the `@cluster` / `@cluster(N)` syntax note.
  - Add a pointer to `bash scripts/verify-m048-s05.sh` as the assembled M048 tooling-truth replay.

- **VS Code README**
  - Remove the `across files` overclaim.
  - Expand syntax wording to mention decorator-position `@cluster` / `@cluster(N)` plus both interpolation forms.
  - Expand proof wording to mention the manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`.
  - Optionally add the short installed-toolchain update note in Installation, but keep the README scoped to VS Code install/packaging/run path.

### Task 2 — Fast fail-closed contract test

Recommended file:
- `scripts/tests/verify-m048-s05-contract.test.mjs`

Why a new dedicated test is better than widening older helpers:
- M034 helper is about installer/public HTTP release surfaces.
- M036 contract test is about support tiers and editor README layering.
- S05 needs a **new milestone-specific contract**: update commands, optional entrypoint, package archive note, VS Code wording correction, and the presence of the new assembled verifier.

What the new contract test should assert:
- `README.md` includes:
  - `meshc update`
  - `meshpkg update`
  - `[package].entrypoint`
  - `bash scripts/verify-m048-s05.sh`
- `website/docs/docs/tooling/index.md` includes:
  - `meshc update`
  - `meshpkg update`
  - `canonical installer path`
  - `[package].entrypoint`
  - `lib/start.mpl`
  - `meshpkg publish`
  - nested-source/archive wording
  - `bash scripts/verify-m048-s05.sh`
  - `@cluster`
- `tools/editors/vscode-mesh/README.md` includes:
  - `@cluster`
  - `@cluster(N)` or `@cluster(3)` wording
  - `mesh.toml`
  - `lib/start.mpl`
  - override-entry proof wording
- `tools/editors/vscode-mesh/README.md` omits:
  - `jump to definitions across files`

If the planner prefers the heavier pattern, the same assertions can instead live in `compiler/meshc/tests/e2e_m048_s05.rs`, but the Node test is likely the lighter sufficient fit for this slice.

### Task 3 — Assembled verifier

Recommended file:
- `scripts/verify-m048-s05.sh`

Recommended behavior:
- use the M047 closeout shell pattern (`status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`)
- fail fast on the S05 contract test before long-running commands
- replay the retained rails rather than inventing new product checks
- keep artifacts under `.tmp/m048-s05/verify`

Recommended phase list:
1. `docs-contract` — `node --test scripts/tests/verify-m048-s05-contract.test.mjs`
2. `m048-s01-entrypoint` — `cargo test -p meshc --test e2e_m048_s01 -- --nocapture`
3. `m048-s02-lsp-neovim` — `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp`
4. `m048-s02-vscode` — `npm --prefix tools/editors/vscode-mesh run test:smoke`
5. `m048-s02-publish` — either:
   - exact S02 replay: `cargo test -p meshpkg -- --nocapture`, or
   - focused publish-only filter if the planner wants to avoid re-running `update_cli`
6. `m048-s03-toolchain-update-core` — `cargo test -p mesh-pkg --test toolchain_update -- --nocapture`
7. `m048-s03-toolchain-update-help` — `cargo test -p meshc --test tooling_e2e test_update -- --nocapture`
8. `m048-s03-toolchain-update-cli` — `cargo test -p meshpkg --test update_cli -- --nocapture`
9. `m048-s03-toolchain-update-e2e` — `cargo test -p meshc --test e2e_m048_s03 -- --nocapture`
10. `m048-s04-shared-grammar` — `bash scripts/verify-m036-s01.sh`
11. `m048-s04-neovim-syntax` — `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`
12. `m048-s04-neovim-contract` — `node --test scripts/tests/verify-m036-s02-contract.test.mjs`
13. `m048-s04-skill-contract` — `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`
14. `docs-build` — `npm --prefix website run build`

## Verification Plan

For implementation work, verify in this order:

1. `node --test scripts/tests/verify-m048-s05-contract.test.mjs`
2. `cargo test -p meshc --test e2e_m048_s01 -- --nocapture`
3. `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp`
4. `npm --prefix tools/editors/vscode-mesh run test:smoke`
5. S03 update command set:
   - `cargo test -p mesh-pkg --test toolchain_update -- --nocapture`
   - `cargo test -p meshc --test tooling_e2e test_update -- --nocapture`
   - `cargo test -p meshpkg --test update_cli -- --nocapture`
   - `cargo test -p meshc --test e2e_m048_s03 -- --nocapture`
6. S04 syntax/skill set:
   - `bash scripts/verify-m036-s01.sh`
   - `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`
   - `node --test scripts/tests/verify-m036-s02-contract.test.mjs`
   - `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`
7. `npm --prefix website run build`
8. full replay: `bash scripts/verify-m048-s05.sh`

## Watchouts

- **Do not build S05 around `scripts/verify-m036-s03.sh`.** It hardcodes the repo-local vendor Neovim binary `.tmp/m036-s02/vendor/nvim-macos-arm64/bin/nvim`, and that file is currently absent here. The validated S02/S04 commands use `NEOVIM_BIN="${NEOVIM_BIN:-nvim}"` instead.

- **VS Code smoke needs `target/debug/meshc`.** `tools/editors/vscode-mesh/src/test/runTest.ts` fails early if the repo-local meshc binary is missing. Run at least one cargo phase before the VS Code smoke.

- **Existing docs tests are exact-string-heavy.** `scripts/lib/m034_public_surface_contract.py`, `scripts/tests/verify-m036-s03-contract.test.mjs`, and `scripts/verify-m034-s04-extension.sh` all inspect README/tooling/VS Code doc markers. Add new text by appending small truthful notes; avoid rewriting existing release/install/VSIX wording unless the associated tests are updated.

- **Artifact retention shape differs by rail.**
  - `.tmp/m048-s01/*` and `.tmp/m048-s03/*` are timestamped buckets.
  - `.tmp/m036-s02/syntax`, `.tmp/m036-s02/lsp`, and `.tmp/m036-s03/vscode-smoke` are fixed directories.
  - If S05 wants a retained bundle, copy fixed M036 directories directly and use snapshot-or-copy logic only for the timestamped M048 buckets.

- **Default `main.mpl` remains correct.** Public docs should present `[package].entrypoint` as an optional override, not a replacement for the default scaffold.

- **Do not widen scope into Neovim or skill content unless tests force it.** Those surfaces already have dedicated retained contract rails and read as truthful from the current repo state.
