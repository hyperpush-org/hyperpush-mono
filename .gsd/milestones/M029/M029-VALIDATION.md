---
verdict: pass
remediation_round: 0
---

# Milestone Validation: M029

## Success Criteria Checklist
- [x] `meshc fmt --check reference-backend` passes with correct `Api.Router`-style dot-paths — evidence: S01 UAT proved `cargo run -q -p meshc -- fmt --check reference-backend` plus `rg -n '^from .*\. ' reference-backend -g '*.mpl'` returned no matches; S03 UAT reran the formatter-clean gate and kept `reference-backend/api/health.mpl` as the canonical multiline-import anchor.
- [x] `meshc fmt --check mesher` passes with 0 files needing reformatting — evidence: S03 summary/UAT proved `cargo run -q -p meshc -- fmt --check mesher` passes and the captured success log `/tmp/m029-s03-fmt-mesher.log` stays empty.
- [x] Both `reference-backend/` and `mesher/` build clean — evidence: S02 summary/UAT proved `cargo run -q -p meshc -- build mesher`; S03 summary/UAT proved both `cargo run -q -p meshc -- build mesher` and `cargo run -q -p meshc -- build reference-backend` pass.
- [x] `cargo test -p meshc --test e2e` shows 318+ pass with no regressions — evidence: validation rerun `cargo test -p meshc --test e2e -- --nocapture` reported `318 passed; 10 failed`; the 10 failures are the known pre-existing `try_*` / `from_try_*` failures recorded in project knowledge, so M029 did not introduce new regressions.
- [x] Zero `<>` remain in `mesher/` except the designated SQL/DDL keep sites — evidence: validation `rg -n '<>' mesher -g '*.mpl'` returned only `mesher/storage/schema.mpl:11-13` plus the two SQL-adjacent `mesher/storage/queries.mpl` sites (`date_trunc(...)` bucket SQL and `DROP TABLE IF EXISTS`), matching the roadmap intent even though later edits shifted the exact line numbers.
- [x] All `List.map(rows, fn...)` wrapping patterns in Mesher were converted to pipe style — evidence: validation `rg -n 'List\.map\(rows,|Ok\(List\.map\(' mesher -g '*.mpl'` returned no matches; S02 summary/UAT documented the four `rows |> List.map(...)` rewrites in `mesher/storage/queries.mpl`.
- [x] All Mesher import lines over 120 chars now use parenthesized multiline form — evidence: S03 summary/UAT documented the rollout in `mesher/main.mpl`, `mesher/ingestion/routes.mpl`, `mesher/api/{alerts,dashboard,team}.mpl`, and `mesher/services/{project,user}.mpl`; validation `rg -n '^from .{121,}' mesher -g '*.mpl'` returned no matches.
- [x] `reference-backend/` source has correct dot-paths (`Api.Router`, not `Api. Router`) — evidence: S01 exact-output formatter regressions plus backend repair proved the fix; validation `rg -n '^from .*\. ' mesher reference-backend -g '*.mpl'` returned no matches across both dogfood apps.

## Slice Delivery Audit
| Slice | Claimed | Delivered | Status |
|-------|---------|-----------|--------|
| S01 | Fix `meshc fmt` dot-path corruption and preserve parenthesized multiline imports, then leave `reference-backend/` formatter-clean. | Summary/UAT show the dedicated `walk_path(...)` formatter path, new exact-output walker/CLI regressions, multiline-import preservation, repaired backend imports, `mesh-fmt --lib` green, `e2e_fmt` green, `e2e_multiline_import_paren` green, and `fmt --check reference-backend` plus no spaced-dot grep matches. | pass |
| S02 | Replace Mesher JSON `<>` chains with `json {}` / interpolation where appropriate, convert wrapping `List.map(rows, ...)` calls to pipe style, and keep Mesher building. | Summary/UAT show `alerts/detail/search` serializer cleanup, the four `rows |> List.map(...)` storage rewrites, the two token-builder interpolation rewrites, only the five accepted SQL/DDL `<>` sites remaining, and `meshc build mesher` green. | pass |
| S03 | Convert remaining overlong Mesher imports to canonical multiline form and close final formatter compliance on both dogfood apps. | Summary/UAT show the manual import-shape rollout, the late formatter/CLI repairs (`pub type`, `table \"...\"`, silent-success `fmt --check`), both formatter checks green, both builds green, no overlong imports, no spaced dot-paths, and a final artifact-driven UAT for the closeout surface. | pass |

## Cross-Slice Integration
- S01’s formatter/path fixes were consumed exactly as planned by S03: the Mesher multiline-import rollout used the repaired formatter behavior and the `reference-backend/api/health.mpl` multiline import remained the canonical anchor throughout.
- S02 left Mesher in the expected pre-S03 state: JSON/interpolation cleanup and pipe-style cleanup stayed intact while S03 handled only import-shape and formatter-closeout work.
- No boundary mismatch remains. The only notable integration wrinkle was expected: S02 warned its exact `file:line` `<>` proof would shift if S03 edited above the keep sites. Validation confirms the allowed-site set is still exactly the roadmap’s SQL/DDL set.
- S03 expanded into additional formatter/CLI fixes when the closeout gates exposed real defects. That is a documented deviation, not a contract miss: it strengthened the roadmap proof surface instead of bypassing it.

## Requirement Coverage
- **R024** — covered by S02 + S03 and validated by S03 UAT: the JSON/interpolation cleanup, pipe cleanup, multiline-import rollout, `meshc fmt --check mesher`, and `meshc build mesher` all closed green.
- **R026** — covered by S01 and reinforced by S03: formatter/library/CLI regressions now guard dotted-path and multiline-import preservation at both the walker and `meshc fmt` layers.
- **R027** — re-scoped into S01 and sustained by S03: `reference-backend/` was repaired to canonical dotted imports and remains formatter-clean with no spaced-dot regressions.
- **R011** — partially covered as planned by the milestone. S03’s UAT explicitly proves the late dogfood friction was fixed in Mesh tooling itself rather than papered over in app code or artifacts.
- No in-scope requirement is left unaddressed for M029.

## Verdict Rationale
M029 passes. Every roadmap success criterion now has direct supporting evidence from the slice summaries/UATs or the validator’s own closeout rerun. The only milestone-definition-of-done gap left by the slice artifacts was the broader `cargo test -p meshc --test e2e` gate, and the validation rerun closed that gap with the expected baseline result: `318 passed; 10 failed`, where the 10 failures are the already-known pre-existing `try_*` / `from_try_*` compiler/runtime issues recorded in project knowledge. No new formatter, Mesher cleanup, or reference-backend regressions surfaced during reconciliation.
