---
verdict: needs-remediation
remediation_round: 0
---

# Milestone Validation: M032

## Success Criteria Checklist
- [x] Criterion 1 — evidence: `mesher/` no longer carries disproven limitation comments for capabilities Mesh already supports. S03 removed the stale request/query/handler folklore from `mesher/ingestion/routes.mpl`, `mesher/services/user.mpl`, and `mesher/services/stream_manager.mpl`; S04 removed the stale module-boundary `from_json` folklore from `mesher/services/event_processor.mpl` and `mesher/storage/queries.mpl`; S05 records the negative stale-folklore sweep and retained keep-site sweep as the final truth surface.
- [x] Criterion 2 — evidence: at least one real blocker found through the audit was fixed in Mesh and then used directly from `mesher/`. S02 fixed the imported inferred-polymorphic export blocker (`xmod_identity` / `m032_inferred_*`) and dogfooded the repaired path by exporting `flush_batch(...)` from `mesher/storage/writer.mpl` and importing it from `mesher/services/writer.mpl`; S02 UAT replays the named tests and Mesher fmt/build gates.
- [x] Criterion 3 — evidence: a short retained-limit ledger remains for the still-real gaps, with each retained comment tied to current evidence rather than folklore. S05 summary and UAT explicitly enumerate the retained Mesh keep-sites (route closures, nested `&&`, `Timer.send_after`, parser-bound case-arm extraction) and the M033 follow-on families (`ORM boundary`, `PARTITION BY`, row-shape `from_json` notes).
- [x] Criterion 4 — evidence: `cargo run -q -p meshc -- fmt --check mesher` and `cargo run -q -p meshc -- build mesher` still pass after cleanup. These gates are recorded as passing in S02, S03, S04, and S05, with S05 replaying them as part of the integrated closeout matrix.

## Slice Delivery Audit
| Slice | Claimed | Delivered | Status |
|-------|---------|-----------|--------|
| S01 | Limitation truth audit and repro matrix | S01 summary substantiates the stale-vs-real matrix, named proof surfaces, and downstream handoff into S02/S03/S04/S05. But `S01-UAT.md` is still a doctor placeholder (`# S01: Recovery placeholder UAT`) rather than a real acceptance script. | needs-remediation |
| S02 | Cross-module and inferred-export blocker retirement | S02 summary and UAT substantiate the real compiler repair, named regression tests, Mesher dogfood move of `flush_batch(...)`, and green fmt/build replay. | pass |
| S03 | Request, handler, and control-flow dogfood cleanup | S03 summary and UAT substantiate the direct `Request.query(...)`, inline service-call `case`, inline cast-handler `if/else`, removal of stale comments, and preservation of real keep-sites. | pass |
| S04 | Module-boundary JSON and workaround convergence | S04 summary and UAT substantiate removal of stale module-boundary `from_json` folklore, retention of the real JSONB/ORM boundary, and green Mesher proof on the cleaned codebase. | pass |
| S05 | Integrated mesher proof and retained-limit ledger | S05 summary and UAT substantiate the integrated replay bundle, the supported-now versus retained-limit ledger, required closeout artifacts, and the M033 handoff map. | pass |

## Cross-Slice Integration
- S01 → S02 aligns: S02 starts from the exact S01 blocker handoff (`xmod_identity`) and closes it with a compiler repair plus Mesher dogfood.
- S01 → S03 aligns: S03 cleans the exact stale request/handler/control-flow families S01 classified as safe to retire, while preserving the keep-sites S01 marked as still real.
- S01 + S02 → S04 aligns: S04 rewrites the mixed-truth `from_json` comment family without disturbing the repaired cross-module path from S02.
- S02 + S03 + S04 → S05 aligns: S05 closes on the supported-now proofs and retained keep-sites those slices established.
- Boundary gap: the S01 handoff exists in the summary and is clearly consumed by later slices, but the slice still lacks a real UAT artifact. The implementation boundaries reconcile; the evidence bundle does not.

## Requirement Coverage
- R010 — covered and validated by S05. The final closeout bundle ties Mesher-backed evidence to the broader deploy/backend-development claim.
- R011 — covered across S01–S05 and validated in `REQUIREMENTS.md`; the slice chain stays anchored to real Mesher friction rather than speculative compiler work.
- R013 — covered and validated by S02, then replayed by S05. The old blocker was fixed in Mesh and dogfooded back into Mesher.
- R035 — covered across S01, S03, S04, and S05 and validated in `REQUIREMENTS.md`; the remaining limitation comments are now tied to current proof.
- No active requirement from M032’s coverage set is left unaddressed. The blocking gap is slice-artifact completeness, not contract coverage.

## Verdict Rationale
Verdict: `needs-remediation`. All four roadmap success criteria are substantiated, the slice-to-slice implementation handoffs line up, and the covered requirements are addressed. The milestone still fails its own definition of done because not all slice deliverables are complete: `S01-UAT.md` is a recovery placeholder instead of a real acceptance script. That is a material evidence gap for milestone sealing, so M032 should not be marked complete until the S01 acceptance artifact is backfilled.

## Remediation Plan
Add and execute this remediation slice before rerunning validation:

- [ ] **S06: S01 acceptance artifact backfill** `risk:low` `depends:[S01]`
  > After this: S01 has a real artifact-driven UAT script derived from its proof matrix instead of a doctor placeholder, so M032 can close with complete slice evidence.
