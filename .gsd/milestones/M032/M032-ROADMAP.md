# M032: Mesher Limitation Truth & Mesh Dogfood Retirement

**Vision:** Audit `mesher/` for claimed Mesh limitations, prove which ones are stale versus still real, fix the real blockers in Mesh itself, and dogfood those repaired paths back into `mesher/` without changing mesher's product behavior.

## Success Criteria

- `mesher/` no longer carries disproven limitation comments for capabilities Mesh already supports.
- At least one real blocker found through the audit is fixed in Mesh and then used directly from `mesher/`.
- A short retained-limit ledger remains for the still-real gaps, with each retained comment tied to current evidence rather than folklore.
- `cargo run -q -p meshc -- fmt --check mesher` and `cargo run -q -p meshc -- build mesher` still pass after the cleanup.

## Key Risks / Unknowns

- Cross-module inferred exports may still fail in real `meshc build` paths even though nearby e2e coverage exists — this matters because stale comments and real bugs may be mixed inside the same workaround family.
- Handler and case-arm workaround comments may reflect behavior that changed unevenly across parser, codegen, and CLI paths — this matters because removing the workaround in mesher without full proof could regress the app.
- Multiple workaround comments may point at one root cause, or one comment may hide several bugs — this matters because the milestone should fix root causes rather than perform cosmetic cleanup.

## Proof Strategy

- Cross-module inferred export and module-boundary ambiguity → retire in S02 by proving the failing build path, fixing the root cause in Mesh, and adding regression coverage plus mesher dogfood.
- Handler, request, and case-arm folklore cleanup → retire in S03 by proving the supported patterns in fresh compiler/CLI repros and replacing stale mesher workaround structure with direct usage.
- Cleanup truthfulness drift → retire in S05 by proving mesher still builds/formats/tests cleanly and every retained limitation note points at current evidence.

## Verification Classes

- Contract verification: targeted `meshc` CLI repros, compiler e2e tests, grep checks over limitation comments, artifact checks over changed mesher modules
- Integration verification: `cargo run -q -p meshc -- build mesher`, `cargo run -q -p meshc -- fmt --check mesher`, and selected mesher migration/build smoke commands on the cleaned codebase
- Operational verification: package-level mesher flow remains green after dogfooding repaired Mesh behavior; no extra operational surface beyond the existing mesher build/migrate lifecycle
- UAT / human verification: none

## Milestone Definition of Done

This milestone is complete only when all are true:

- all slice deliverables are complete
- stale limitation comments and stale workaround structure have been retired from the audited mesher surfaces
- at least one still-real blocker has been fixed in Mesh and dogfooded back into mesher
- the real entrypoint (`meshc` against `mesher/`) exists and is exercised
- success criteria are re-checked against live behavior, not just code diffs
- final integrated acceptance scenarios pass

## Requirement Coverage

- Covers: R011, R013, R035
- Partially covers: R010
- Leaves for later: R036, R037, R038, R039, R040, R041
- Orphan risks: none

## Slices

- [x] **S01: Limitation truth audit and repro matrix** `risk:high` `depends:[]`
  > After this: the audited `mesher/` limitation comments are classified as stale versus real with concrete repro commands and affected module clusters.

- [x] **S02: Cross-module and inferred-export blocker retirement** `risk:high` `depends:[S01]`
  > After this: at least one live compiler or runtime blocker behind a mesher workaround is fixed in Mesh, regression-covered, and used from mesher.

- [x] **S03: Request, handler, and control-flow dogfood cleanup** `risk:medium` `depends:[S01]`
  > After this: mesher uses currently supported request and handler patterns directly in selected audited modules, with stale workaround comments removed.

- [ ] **S04: Module-boundary JSON and workaround convergence** `risk:medium` `depends:[S01,S02]`
  > After this: a validated module-boundary workaround family is simplified in mesher to the repaired or already-supported Mesh path without product drift.

- [ ] **S05: Integrated mesher proof and retained-limit ledger** `risk:low` `depends:[S02,S03,S04]`
  > After this: mesher build/format proof passes on the cleaned codebase and the remaining limitation comments form a short verified keep-list that hands cleanly into M033.

## Boundary Map

### S01 → S02

Produces:
- audited limitation matrix for `mesher/` modules, grouped into stale-vs-real claims with concrete repro commands
- verified list of live blocker candidates with likely owning Mesh subsystems (parser, typechecker, codegen, runtime, tooling)
- comment and grep baselines for the audited workaround families

Consumes:
- nothing (first slice)

### S01 → S03

Produces:
- verified list of stale request, handler, and control-flow workaround comments that can be retired safely
- proof snippets showing which currently supported patterns already compile or run through real CLI paths
- target module list for direct dogfood cleanup in `mesher/`

Consumes:
- nothing (first slice)

### S02 → S04

Produces:
- repaired Mesh behavior for at least one real blocker family with regression coverage
- mesher-ready usage pattern for the repaired path
- narrowed root-cause map for remaining module-boundary workaround families

Consumes from S01:
- live blocker matrix and affected module inventory

### S03 → S05

Produces:
- cleaned mesher modules using current request and handler behavior directly
- removed stale comments and proof that product behavior stayed stable in those audited paths
- updated grep expectations for limitation folklore

Consumes from S01:
- stale-comment classification and direct-cleanup target list

### S04 → S05

Produces:
- mesher module-boundary cleanup on top of repaired Mesh behavior
- updated regression coverage and any remaining verified limitation notes that still survive the milestone
- handoff list of true ORM or migration follow-on gaps for M033

Consumes from S01:
- workaround family classification and repro matrix

Consumes from S02:
- repaired Mesh path and regression surface
