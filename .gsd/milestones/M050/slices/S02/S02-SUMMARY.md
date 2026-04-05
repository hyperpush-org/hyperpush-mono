---
id: S02
parent: M050
milestone: M050
provides:
  - A coherent install -> hello-world -> starter chooser across README, Getting Started, Clustered Example, and Tooling.
  - A focused first-contact docs verifier (`bash scripts/verify-m050-s02.sh`) that proves both source markers and built-site HTML output.
  - An assembled onboarding replay (`bash scripts/verify-m049-s05.sh`) that now runs the M050 docs preflights first and refreshes local fallback Postgres metadata when needed.
requires:
  - slice: S01
    provides: Primary onboarding graph placement, proof-surface sidebar/footer rules, and the fast docs-graph preflight via `bash scripts/verify-m050-s01.sh`.
affects:
  - S03
key_files:
  - README.md
  - website/docs/docs/getting-started/index.md
  - website/docs/docs/getting-started/clustered-example/index.md
  - website/docs/docs/tooling/index.md
  - scripts/tests/verify-m050-s02-first-contact-contract.test.mjs
  - scripts/verify-m050-s02.sh
  - scripts/verify-m049-s05.sh
  - scripts/verify-m047-s04.sh
  - scripts/fixtures/m036-s01-syntax-corpus.json
  - compiler/meshc/tests/e2e_m050_s02.rs
  - compiler/meshc/tests/e2e_m049_s05.rs
  - .gsd/PROJECT.md
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Keep retained clustered verifier names on Distributed Proof and Tooling, not on Clustered Example.
  - Use `bash scripts/verify-m050-s02.sh` as the focused first-contact docs verifier and replay it immediately after `bash scripts/verify-m050-s01.sh` inside `bash scripts/verify-m049-s05.sh`.
  - Treat stale fallback local Postgres metadata as verifier-owned state that `scripts/verify-m049-s05.sh` should refresh, not as product/docs drift.
patterns_established:
  - First-contact docs contracts should pin starter commands, current repo links, and ordering markers while leaving paragraph-level wording flexible.
  - Historical shell verifiers must follow the newer source-level Rust/docs contract splits instead of forcing proof-heavy copy back onto first-contact pages.
  - Docs-backed syntax corpora should be repaired by retargeting audited line ranges, not by padding docs or weakening grammar rails.
observability_surfaces:
  - `.tmp/m050-s02/verify/status.txt`, `.tmp/m050-s02/verify/phase-report.txt`, and `.tmp/m050-s02/verify/built-html/summary.json` for first-contact docs truth.
  - `.tmp/m049-s05/verify/status.txt`, `.tmp/m049-s05/verify/current-phase.txt`, and `.tmp/m049-s05/verify/phase-report.txt` for the assembled onboarding replay.
  - `.tmp/m047-s04/verify/phase-report.txt` and `.tmp/m048-s05/verify/m048-s04-shared-grammar.log` for retained-wrapper drift.
  - `.tmp/m049-s05/verify/m049-s01-env-preflight.*` when the fallback local Postgres source needs repair.
drill_down_paths:
  - .gsd/milestones/M050/slices/S02/tasks/T01-SUMMARY.md
  - .gsd/milestones/M050/slices/S02/tasks/T02-SUMMARY.md
  - .gsd/milestones/M050/slices/S02/tasks/T03-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-04T03:11:56.594Z
blocker_discovered: false
---

# S02: First-Contact Docs Rewrite

**Mesh’s public first-contact docs now teach one honest starter chooser — clustered, SQLite-local, or Postgres-shared — and that story is enforced by a dedicated M050 verifier plus the retained assembled M049 replay.**

## What Happened

S02 rewrote the public first-contact story across README, Getting Started, Clustered Example, and Tooling so Mesh now teaches one coherent evaluator path: install Mesh, run hello-world, then deliberately choose `meshc init --clustered`, `meshc init --template todo-api --db sqlite`, or `meshc init --template todo-api --db postgres`. The clustered page stays scaffold-first and hands retained proof discoverability off to Distributed Proof instead of listing historical rails inline, while Tooling now exposes the new first-contact docs verifier before the broader assembled onboarding verifier. Closeout also repaired the retained wrapper stack these pages still feed: `scripts/verify-m049-s05.sh` now refreshes fallback local Postgres metadata when the container-backed source has drifted, `scripts/verify-m047-s04.sh` now matches the lighter Clustered Example contract, and the shared interpolation syntax corpus was retargeted to current docs-backed snippet lines so the retained M036/M048 grammar rails stayed green.

## Verification

Passed the slice-owned docs rails and the retained assembled replay: `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`, `node --test scripts/tests/verify-m048-s05-contract.test.mjs`, `node --test scripts/tests/verify-m036-s03-contract.test.mjs`, `node --test scripts/tests/verify-m049-s05-contract.test.mjs`, `cargo test -p meshc --test e2e_m050_s02 -- --nocapture`, `cargo test -p meshc --test e2e_m049_s05 -- --nocapture`, `cargo test -p meshc --test e2e_m047_s04 -- --nocapture`, `bash scripts/verify-m036-s01.sh`, `bash scripts/verify-m048-s05.sh`, `bash scripts/verify-m050-s02.sh`, and the final authoritative `bash scripts/verify-m049-s05.sh` replay. The retained bundle now lands at `.tmp/m049-s05/verify/retained-proof-bundle/` with the M050 docs preflights passing before the older M039/M045/M047/M048 replays.

## Requirements Advanced

None.

## Requirements Validated

None.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

Closeout required three retained-verifier repairs beyond the original task list: `scripts/verify-m049-s05.sh` now refreshes fallback local Postgres container metadata before replaying `e2e_m049_s01`, `scripts/verify-m047-s04.sh` was aligned with the lighter Clustered Example contract so it no longer forces proof-rail commands onto the first-contact page, and `scripts/fixtures/m036-s01-syntax-corpus.json` was retargeted to the current docs-backed interpolation lines so the retained M036/M048 grammar rails stayed truthful.

## Known Limitations

S03 still needs the deeper proof-page reconciliation so the low-level/runtime-owned split is consistent across all secondary docs surfaces, not just README / Getting Started / Clustered Example / Tooling. The docs-backed interpolation syntax corpus still uses absolute Markdown line ranges, so future copy edits can require corpus updates even when the grammar is unchanged. The full assembled `bash scripts/verify-m049-s05.sh` replay also still depends on local retained-rail prerequisites such as Docker, Neovim, and the VS Code smoke environment.

## Follow-ups

S03 should reconcile the remaining proof-heavy pages (`Distributed Proof`, `Distributed Actors`, and `Production Backend Proof`) so the low-level/runtime-owned split stays clear without leaking proof-map density back into first-contact surfaces. If Tooling or proof-page wording changes again, update `scripts/verify-m047-s04.sh`, `scripts/verify-m047-s06.sh`, and the first-contact contract tests together so the retained wrappers continue to match the lighter public split.

## Files Created/Modified

- `README.md` — Rewrote the root public starter story around the clustered / SQLite / Postgres chooser and the current repo URL.
- `website/docs/docs/getting-started/index.md` — Aligned Getting Started with the same post-hello-world starter split and proof-secondary next steps.
- `website/docs/docs/getting-started/clustered-example/index.md` — Kept Clustered Example scaffold-first, preserved the explicit follow-on starter section, and linked outward to Distributed Proof instead of listing retained rails inline.
- `website/docs/docs/tooling/index.md` — Reordered Tooling around install/update/project creation and exposed the new S02 verifier plus the assembled M049 verifier.
- `scripts/tests/verify-m050-s02-first-contact-contract.test.mjs` — Added fail-closed source-level contract coverage for README, Getting Started, Clustered Example, and Tooling.
- `scripts/verify-m050-s02.sh` — Added the slice-owned source/build verifier and retained built HTML evidence bundle.
- `scripts/verify-m049-s05.sh` — Replays the M050 docs preflights first and now refreshes fallback local Postgres metadata before rerunning the retained M049 Postgres rail.
- `scripts/verify-m047-s04.sh` — No longer forces retained proof-rail command names onto Clustered Example; keeps that requirement on proof-heavy pages instead.
- `scripts/fixtures/m036-s01-syntax-corpus.json` — Retargeted docs-backed interpolation corpus cases to the real current snippet lines in Language Basics and Cheatsheet.
- `compiler/meshc/tests/e2e_m047_s05.rs` — Kept the retained docs/verifier contracts aligned with the lighter first-contact split.
- `compiler/meshc/tests/e2e_m047_s06.rs` — Kept the retained docs/verifier contracts aligned with the lighter first-contact split.
- `compiler/meshc/tests/e2e_m050_s02.rs` — Pins the new S02 verifier contract and built-HTML bundle shape.
- `compiler/meshc/tests/e2e_m049_s05.rs` — Pins the assembled M049 wrapper order and retained M050 bundle markers.
- `.gsd/PROJECT.md` — Updated project current-state text to reflect M050/S02 completion and the new docs verifier stack.
- `.gsd/KNOWLEDGE.md` — Recorded verifier-resume lessons for stale fallback Postgres metadata and docs-backed syntax corpus drift.
