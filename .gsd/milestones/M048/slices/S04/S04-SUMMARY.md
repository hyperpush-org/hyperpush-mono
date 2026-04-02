---
id: S04
parent: M048
milestone: M048
provides:
  - A shared `@cluster` decorator oracle (`scripts/fixtures/m048-s04-cluster-decorators.mpl`) reused by the retained TextMate/Shiki and Neovim syntax rails.
  - Decorator-only VS Code/TextMate and Neovim/Vim highlighting that proves `@cluster` / `@cluster(3)` without reserving bare `cluster`, while keeping both interpolation forms covered.
  - A dedicated `tools/skill/mesh/skills/clustering/SKILL.md` guide plus a retained skill-contract rail that keeps the auto-loaded Mesh bundle aligned with the current source-first clustered runtime story.
requires:
  - slice: S02
    provides: Manifest-first editor-host rails, Neovim contract tests, and the broader editor-truth contract that S04 extended with syntax parity instead of replacing.
affects:
  - S05
key_files:
  - scripts/fixtures/m048-s04-cluster-decorators.mpl
  - website/scripts/tests/verify-m036-s01-syntax-parity.test.mjs
  - tools/editors/vscode-mesh/syntaxes/mesh.tmLanguage.json
  - tools/editors/neovim-mesh/syntax/mesh.vim
  - tools/editors/neovim-mesh/tests/smoke.lua
  - tools/editors/neovim-mesh/README.md
  - scripts/tests/verify-m036-s02-contract.test.mjs
  - tools/skill/mesh/SKILL.md
  - tools/skill/mesh/skills/clustering/SKILL.md
  - tools/skill/mesh/skills/syntax/SKILL.md
  - tools/skill/mesh/skills/http/SKILL.md
  - scripts/tests/verify-m048-s04-skill-contract.test.mjs
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
  - .gsd/PROJECT.md
key_decisions:
  - D324: prove `@cluster` editor parity through one shared decorator fixture reused by the retained TextMate/Shiki and Neovim rails instead of widening the older interpolation corpus.
  - Keep `cluster` special-cased only when anchored by `@` so bare `cluster` remains a valid identifier in both editor grammars.
  - D325: centralize clustered-runtime teaching in `tools/skill/mesh/skills/clustering/SKILL.md` and keep syntax/http skills as bounded cross-links guarded by `scripts/tests/verify-m048-s04-skill-contract.test.mjs`.
patterns_established:
  - When a syntax addition is smaller than the existing corpus contract, add a dedicated shared fixture with explicit positive and negative probes instead of rewriting the older corpus manifest.
  - Keep grammar, README, and skill-bundle truth on retained contract tests with precise per-file or per-range failure messages so wording and highlighting drift fail closed.
  - For first-contact Mesh guidance, centralize the clustered-runtime story in one dedicated sub-skill and let narrower syntax/HTTP skills cross-link it instead of duplicating the full contract.
observability_surfaces:
  - The TextMate/Shiki parity rail now emits fixture-local case ids and source ranges for decorator drift (`plain-decorator-cluster`, `counted-decorator-count`, `bare-cluster-identifier`).
  - The Neovim smoke emits named file/line/column probes (`plain-decorator-name`, `counted-decorator-count`, `bare-cluster-identifier`) so Vim syntax drift localizes to exact positions instead of vague highlighting failures.
  - `scripts/tests/verify-m048-s04-skill-contract.test.mjs` fails with per-file assertion messages when clustered guidance disappears or stale legacy tokens re-enter the auto-loaded Mesh skill bundle.
drill_down_paths:
  - .gsd/milestones/M048/slices/S04/tasks/T01-SUMMARY.md
  - .gsd/milestones/M048/slices/S04/tasks/T02-SUMMARY.md
  - .gsd/milestones/M048/slices/S04/tasks/T03-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-02T17:55:09.113Z
blocker_discovered: false
---

# S04: Syntax and init-skill parity reset

**Aligned the official Mesh editor grammars and init-time skill bundle to the current `@cluster` / interpolation / clustered-runtime contract with retained drift rails.**

## What Happened

S04 closed the remaining first-contact parity gap that S02 intentionally left open. T01 added one shared decorator fixture, `scripts/fixtures/m048-s04-cluster-decorators.mpl`, and extended the retained TextMate/Shiki parity harness so the official VS Code/docs grammar now proves `@cluster` and `@cluster(3)` as decorator syntax while the negative `let cluster = 1` case stays an ordinary identifier. The change is deliberately narrow: `cluster` is not promoted to a global keyword, the pre-existing interpolation corpus remains intact, and the retained rail now reports exact fixture ranges and case ids when decorator scopes drift.

T02 carried the same bounded contract through the repo-owned Neovim pack instead of inventing a second syntax story. `tools/editors/neovim-mesh/syntax/mesh.vim` now highlights `@cluster` with a dedicated group, the headless smoke replays the shared fixture after the interpolation corpus loop, and the README plus `scripts/tests/verify-m036-s02-contract.test.mjs` now describe the proof surface truthfully as interpolation corpus plus the shared decorator oracle. Downstream slices can treat the Neovim/Vim path as the same contract with a different host, not a separate one-off implementation.

T03 reset the init-time Mesh skill bundle to the current clustered/runtime story. The root skill now routes clustered/bootstrap/operator questions to a dedicated `skills/clustering` guide, that guide teaches `@cluster`, `@cluster(N)`, `Node.start_from_env()`, scaffold commands, operator CLI commands, and the bounded `HTTP.clustered(...)` story, and the syntax/http sub-skills now cross-link that contract without duplicating or contradicting it. The retained `scripts/tests/verify-m048-s04-skill-contract.test.mjs` rail means future wording drift or stale legacy tokens fail closed instead of silently regressing the first-contact experience.

Assembled together, the slice now delivers one editor/teaching truth surface for downstream work: official grammars and the auto-loaded Mesh skill bundle all match the current source-first clustered runtime contract, both interpolation forms remain proven, and bare `cluster` stays legal user code. S05 can therefore consume retained rails instead of rediscovering syntax semantics or cluster-teaching wording from scratch.

## Verification

All slice-level verification commands now pass.

- `bash scripts/verify-m036-s01.sh` ✅ passed. This replays the compiler-token truth checks plus the shared TextMate/Shiki parity harness and proves the dedicated `@cluster` decorator fixture stays in sync with the existing interpolation contract.
- `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax` ✅ passed. The Neovim/Vim rail replays the interpolation corpus, opens `scripts/fixtures/m048-s04-cluster-decorators.mpl`, and logs exact decorator probes proving `@cluster` / `@cluster(3)` highlight correctly while bare `cluster` remains `meshVariable`.
- `node --test scripts/tests/verify-m036-s02-contract.test.mjs` ✅ passed. The bounded Neovim README/runtime/smoke contract stayed synchronized with the actual proof surface.
- `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs` ✅ passed. The retained skill-bundle rail now proves root-skill routing, clustering sub-skill coverage, syntax/http cross-links, and rejection of stale clustered guidance tokens.

Together these rails validate both halves of the slice goal: editor syntax truth and first-contact clustered/runtime teaching truth.

## Requirements Advanced

- R114 — S04 delivered the remaining parity work promised by the requirement: shared `@cluster` fixture-based editor rails for VS Code/TextMate and Neovim, plus a clustering-aware init-time Mesh skill bundle.

## Requirements Validated

- R114 — `bash scripts/verify-m036-s01.sh`, `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`, `node --test scripts/tests/verify-m036-s02-contract.test.mjs`, and `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs` all passed, proving `@cluster` / `@cluster(N)` highlighting, both interpolation forms, and clustering-aware init-time Mesh skill guidance.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

T01 created `scripts/tests/verify-m048-s04-skill-contract.test.mjs` early as a temporary failing placeholder because the slice verifier already referenced the retained rail before the skill refresh landed in T03. The slice closed green after T03 replaced that placeholder with the real contract test. The editor work also deliberately kept the `3` in `@cluster(3)` on the ordinary integer scopes/groups (`constant.numeric.integer.mesh` / `meshNumberInteger`) instead of inventing a decorator-count-only token; only decorator-position `cluster` is special-cased.

## Known Limitations

Product-side editor and skill surfaces landed as planned. The only remaining closeout limitation is bookkeeping: `R114` is rendered in `.gsd/REQUIREMENTS.md`, but the GSD requirement DB did not resolve that ID during this slice closeout, so D326 is currently the authoritative validation record until the DB entry is reconciled.

## Follow-ups

- S05 should assemble `bash scripts/verify-m036-s01.sh`, `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`, `node --test scripts/tests/verify-m036-s02-contract.test.mjs`, and `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs` into the milestone closeout verifier and the minimal public touchpoints it updates.
- Reconcile the GSD requirement DB entry for `R114` if milestone bookkeeping needs the rendered requirement status to flip automatically; `gsd_requirement_update` could not find the ID during this closeout even though `.gsd/REQUIREMENTS.md` renders it and D326 records the validation evidence.

## Files Created/Modified

- `scripts/fixtures/m048-s04-cluster-decorators.mpl` — Added the shared positive/negative syntax oracle for `@cluster`, `@cluster(3)`, and bare `cluster`.
- `website/scripts/tests/verify-m036-s01-syntax-parity.test.mjs` — Extended the retained TextMate/Shiki parity rail with fixture-specific decorator assertions and token-signature parity.
- `tools/editors/vscode-mesh/syntaxes/mesh.tmLanguage.json` — Anchored special highlighting to decorator-position `@cluster` matches without reserving bare `cluster` globally.
- `tools/editors/neovim-mesh/syntax/mesh.vim` — Added the Neovim decorator syntax rule while leaving counted arity on the ordinary integer group.
- `tools/editors/neovim-mesh/tests/smoke.lua` — Extended the headless Neovim smoke with named decorator probes against the shared fixture.
- `tools/editors/neovim-mesh/README.md` — Documented the bounded Neovim syntax proof surface as interpolation corpus plus the shared `@cluster` fixture.
- `scripts/tests/verify-m036-s02-contract.test.mjs` — Pinned the Neovim README/runtime/smoke contract to the updated bounded syntax surface.
- `tools/skill/mesh/SKILL.md` — Updated the top-level Mesh skill to mention clustered runtime bootstrapping and route questions to the dedicated clustering sub-skill.
- `tools/skill/mesh/skills/clustering/SKILL.md` — Added the canonical clustered-runtime skill covering `@cluster`, `Node.start_from_env()`, scaffold commands, operator commands, and `HTTP.clustered(...)`.
- `tools/skill/mesh/skills/syntax/SKILL.md` — Added decorator-specific syntax guidance and a clustering cross-link for generic syntax questions.
- `tools/skill/mesh/skills/http/SKILL.md` — Added current method-specific route helpers, `HTTP.clustered(...)`, and clustered-boundary guidance while preserving `HTTP.route(...)`.
- `scripts/tests/verify-m048-s04-skill-contract.test.mjs` — Added the retained skill-bundle drift rail for clustered/runtime guidance and stale legacy tokens.
- `.gsd/KNOWLEDGE.md` — Recorded the decorator-fixture oracle and skill-bundle maintenance rules for future agents.
- `.gsd/DECISIONS.md` — Recorded R114 validation evidence in the decisions register (D326) alongside the slice's pattern decisions.
- `.gsd/PROJECT.md` — Refreshed current project state now that M048/S04 is complete and only S05 remains for milestone closeout.
