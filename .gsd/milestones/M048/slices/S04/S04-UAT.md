# S04: Syntax and init-skill parity reset — UAT

**Milestone:** M048
**Written:** 2026-04-02T17:55:09.124Z

# S04 UAT — Syntax and init-skill parity reset

## Preconditions
- Run from the repository root.
- Node dependencies for `website/` are installed so the TextMate/Shiki parity harness can load the repo-pinned grammar tooling.
- `nvim` 0.11+ is available as `${NEOVIM_BIN:-nvim}` for the Neovim smoke.
- No editor-host packaging step is required; the retained rails operate directly against the grammar, Neovim pack, and auto-loaded skill files in this repository.

## Test Case 1 — Shared TextMate/Shiki grammar keeps interpolation parity and decorator-only `@cluster`
1. Run `bash scripts/verify-m036-s01.sh`.
2. If you need case-local output, rerun `node --test website/scripts/tests/verify-m036-s01-syntax-parity.test.mjs`.

Expected outcomes:
- The shared syntax parity suite passes.
- `scripts/fixtures/m048-s04-cluster-decorators.mpl` produces special scopes for `@cluster` and `@cluster(3)`.
- The count in `@cluster(3)` stays on the normal integer scope.
- `let cluster = 1` is tokenized as a variable, not as a decorator or keyword.
- The pre-existing interpolation corpus still passes for both `#{}` and `${}` forms.

## Test Case 2 — Neovim/Vim syntax rail proves the same decorator contract
1. Run `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`.
2. Inspect the emitted `phase=syntax` lines for the shared decorator fixture.

Expected outcomes:
- The verifier ends with `result=pass`.
- The log includes named probes `plain-decorator-name`, `counted-decorator-count`, and `bare-cluster-identifier`.
- `plain-decorator-name` and `counted-decorator-name` report `meshClusterDecorator`.
- `counted-decorator-count` reports `meshNumberInteger`.
- `bare-cluster-identifier` reports `meshVariable`.

## Test Case 3 — Neovim docs/contract text stays synchronized with the bounded proof surface
1. Run `node --test scripts/tests/verify-m036-s02-contract.test.mjs`.

Expected outcomes:
- The test passes.
- `tools/editors/neovim-mesh/README.md` still mentions `@cluster`, the shared interpolation corpus, `scripts/fixtures/m048-s04-cluster-decorators.mpl`, and the manifest-first root contract.
- The smoke runner contract still advertises `decorator_probes=` and the override-entry LSP cases inherited from S02.

## Test Case 4 — Auto-loaded Mesh skill bundle teaches the current clustered-runtime story
1. Run `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`.
2. If the test fails, inspect `tools/skill/mesh/SKILL.md`, `tools/skill/mesh/skills/clustering/SKILL.md`, `tools/skill/mesh/skills/syntax/SKILL.md`, and `tools/skill/mesh/skills/http/SKILL.md`.

Expected outcomes:
- The root Mesh skill routes clustered/bootstrap/operator questions to `skills/clustering`.
- `tools/skill/mesh/skills/clustering/SKILL.md` mentions `@cluster`, `@cluster(N)`, `Node.start_from_env()`, `meshc init --clustered`, `meshc init --template todo-api`, `meshc cluster status|continuity|diagnostics`, and both accepted `HTTP.clustered(...)` forms.
- `tools/skill/mesh/skills/syntax/SKILL.md` and `tools/skill/mesh/skills/http/SKILL.md` cross-link the clustered story without duplicating or contradicting it.
- Legacy tokens such as `[cluster]`, `clustered(work)`, `execute_declared_work`, and `Work.execute_declared_work` are absent.

## Test Case 5 — Regression edge: bare `cluster` remains legal user code while clustered guidance stays current
1. Open `scripts/fixtures/m048-s04-cluster-decorators.mpl`.
2. Confirm it contains `@cluster pub fn add()`, `@cluster(3) pub fn sync_todos()`, and `let cluster = 1`.
3. Re-run all three retained rails:
   - `bash scripts/verify-m036-s01.sh`
   - `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`
   - `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`

Expected outcomes:
- All three commands pass without editing the fixture.
- Any future change that globally reserves bare `cluster` or drops clustered-runtime guidance fails one of these retained rails immediately.

## Edge checks to replay before milestone closeout
- `node --test website/scripts/tests/verify-m036-s01-syntax-parity.test.mjs --test-name-pattern="shared grammar scopes @cluster decorators consistently in both TextMate and Shiki"`
  - Expected: the focused decorator parity test passes and names the fixture-local cases if it fails.
- `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs --test-name-pattern="Mesh skill bundle rejects stale clustered guidance patterns"`
  - Expected: the focused stale-token guard passes for every auto-loaded Mesh skill file.
