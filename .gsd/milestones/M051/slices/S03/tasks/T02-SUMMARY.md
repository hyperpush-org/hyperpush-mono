---
id: T02
parent: S03
milestone: M051
provides: []
requires: []
affects: []
key_files: ["tools/editors/vscode-mesh/src/test/suite/extension.test.ts", "tools/editors/vscode-mesh/out/test/suite/extension.test.js", "tools/editors/neovim-mesh/tests/smoke.lua", "scripts/fixtures/m036-s01-syntax-corpus.json", "compiler/meshc/tests/e2e_m051_s03.rs", ".gsd/KNOWLEDGE.md"]
key_decisions: ["Keep the editor-host proof surface bounded to the retained backend fixture path while preserving the existing same-file definition and manifest-first override-entry behaviors instead of redesigning the smoke harnesses.", "Preserve the shared syntax corpus contract version and case count, and fix source-line drift in place when verification proves a case no longer selects the intended interpolation snippet."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Ran `cargo test -p meshc --test e2e_m051_s03 -- --nocapture` to confirm the slice-owned retained-path contract. Ran `npm --prefix tools/editors/vscode-mesh run test:smoke`, which passed with retained-fixture diagnostics, hover, same-file definition, and override-entry behavior. Ran `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`, which first failed on a stale Mesher corpus line selection and then passed after updating the corpus line range. Ran `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp`, which passed with the retained backend root, override-entry root, single-file mode, and missing-override negative test all intact."
completed_at: 2026-04-04T15:56:39.707Z
blocker_discovered: false
---

# T02: Retargeted the VS Code and Neovim smoke rails plus the shared syntax corpus to the retained backend fixture.

> Retargeted the VS Code and Neovim smoke rails plus the shared syntax corpus to the retained backend fixture.

## What Happened
---
id: T02
parent: S03
milestone: M051
key_files:
  - tools/editors/vscode-mesh/src/test/suite/extension.test.ts
  - tools/editors/vscode-mesh/out/test/suite/extension.test.js
  - tools/editors/neovim-mesh/tests/smoke.lua
  - scripts/fixtures/m036-s01-syntax-corpus.json
  - compiler/meshc/tests/e2e_m051_s03.rs
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Keep the editor-host proof surface bounded to the retained backend fixture path while preserving the existing same-file definition and manifest-first override-entry behaviors instead of redesigning the smoke harnesses.
  - Preserve the shared syntax corpus contract version and case count, and fix source-line drift in place when verification proves a case no longer selects the intended interpolation snippet.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T15:56:39.711Z
blocker_discovered: false
---

# T02: Retargeted the VS Code and Neovim smoke rails plus the shared syntax corpus to the retained backend fixture.

**Retargeted the VS Code and Neovim smoke rails plus the shared syntax corpus to the retained backend fixture.**

## What Happened

Updated the VS Code Extension Development Host smoke to open retained-fixture `api/health.mpl` and `api/jobs.mpl`, kept the same clean-diagnostics, hover, same-file definition, and override-entry assertions, and let the compile step refresh the generated smoke output. Updated the Neovim smoke to reuse a retained backend root constant for the real-project LSP attach, expected root, and missing-override negative test while leaving the override-entry and standalone-file cases intact. Moved the backend-shaped interpolation corpus case onto `scripts/fixtures/backend/reference-backend/main.mpl` and extended `compiler/meshc/tests/e2e_m051_s03.rs` with fail-closed editor/corpus source assertions. During slice verification, the syntax rail exposed an unrelated stale Mesher corpus line range; I corrected that case from line 58 to line 69 without changing the corpus contract version or case count.

## Verification

Ran `cargo test -p meshc --test e2e_m051_s03 -- --nocapture` to confirm the slice-owned retained-path contract. Ran `npm --prefix tools/editors/vscode-mesh run test:smoke`, which passed with retained-fixture diagnostics, hover, same-file definition, and override-entry behavior. Ran `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`, which first failed on a stale Mesher corpus line selection and then passed after updating the corpus line range. Ran `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp`, which passed with the retained backend root, override-entry root, single-file mode, and missing-override negative test all intact.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m051_s03 -- --nocapture` | 0 | ✅ pass | 9351ms |
| 2 | `npm --prefix tools/editors/vscode-mesh run test:smoke` | 0 | ✅ pass | 12200ms |
| 3 | `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax` | 1 | ❌ fail | 1326ms |
| 4 | `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax` | 0 | ✅ pass | 1472ms |
| 5 | `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp` | 0 | ✅ pass | 10818ms |


## Deviations

Updated the unrelated `mesher-connected-peer-log` syntax corpus case from line 58 to line 69 after the T02 syntax verifier proved the shared corpus had drifted. This kept the contract truthful without changing the corpus version or case count.

## Known Issues

None.

## Files Created/Modified

- `tools/editors/vscode-mesh/src/test/suite/extension.test.ts`
- `tools/editors/vscode-mesh/out/test/suite/extension.test.js`
- `tools/editors/neovim-mesh/tests/smoke.lua`
- `scripts/fixtures/m036-s01-syntax-corpus.json`
- `compiler/meshc/tests/e2e_m051_s03.rs`
- `.gsd/KNOWLEDGE.md`


## Deviations
Updated the unrelated `mesher-connected-peer-log` syntax corpus case from line 58 to line 69 after the T02 syntax verifier proved the shared corpus had drifted. This kept the contract truthful without changing the corpus version or case count.

## Known Issues
None.
