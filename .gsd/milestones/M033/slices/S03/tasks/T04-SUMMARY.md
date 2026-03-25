---
id: T04
parent: S03
milestone: M033
key_files:
  - mesher/storage/queries.mpl
key_decisions:
  - Use conditional `Query` assembly plus small Mesh-side composition to retire whole-query read SQL instead of inventing a fake universal SQL abstraction.
  - Stabilize the file in a compileable partial state once the context-budget warning arrived, and leave the remaining red proof families as explicit resume targets rather than starting another unfocused debug cycle.
duration: ""
verification_result: mixed
completed_at: 2026-03-25T20:11:57.391Z
blocker_discovered: false
---

# T04: Stabilized a partial T04 pass with builder-backed issue/event read rewrites and precise resume notes for the remaining S03 regressions

**Stabilized a partial T04 pass with builder-backed issue/event read rewrites and precise resume notes for the remaining S03 regressions**

## What Happened

This unit started from a red S03 gate and re-read the T04 plan plus the live Mesher harness before changing code. A focused repro of `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture` showed the failure was broader than the original alert timeout: unresolved issue listing dropped caller-visible fields, dashboard volume rows lost the `bucket` value, `/api/v1/issues/:issue_id/events` crashed inside the live handler, and the fresh new-issue alert never inserted. Based on that evidence, I avoided more broad re-research and made only the highest-confidence storage-layer rewrites in `mesher/storage/queries.mpl`.

Implemented changes in this pass:
- Added small Mesh-side composition helpers `first_row_value_or`, `count_project_events_in_window`, `get_next_event_id`, and `get_prev_event_id` near the existing read helper section.
- Rewrote `list_issues_filtered(...)` off `Repo.query_raw(...)` and onto conditional `Query` assembly with explicit projected row keys and only the tuple cursor predicate retained as a narrow `Query.where_raw(...)` fragment.
- Rewrote `list_events_for_issue(...)` off the old projection shape onto explicit builder-backed `select_exprs`, preserving the existing paging/order contract while keeping the tuple cursor predicate raw.
- Rewrote `issue_event_timeline(...)` off `Query.select_raw(...)` and onto explicit `select_exprs`.
- Rewrote `project_health_summary(...)` off its whole-query scalar-subquery SQL and onto three simple builder-backed counts plus a one-row Mesh-side map composition.
- Tightened the read-side raw-tail comments so `check_volume_spikes(...)` remains explicitly named as an honest S03 keep-site instead of an anonymous raw leftover.
- Adjusted `event_volume_hourly(...)` to text-cast the computed bucket/count projections so the dashboard route receives stable string-valued map keys for JSON serialization.
- Replaced `is_issue_discarded(...)`’s `Query.select_raw(...)` probe with a plain `id` projection.

I stopped here because the context-budget warning triggered while T04 was still mid-pass. I intentionally did not start another broad debug loop or add the planned `hard_reads` proofs once the warning arrived. Remaining work for the next unit is precise and localized: finish the `get_event_neighbors(...)` rewrite that was prepared but not applied, rewrite `evaluate_threshold_rule(...)`, and then revisit `get_event_alert_rules(...)` / `get_threshold_rules(...)` because the missing fresh-rule alert insert still points at the live alert selector path. After those storage fixes, add the planned `e2e_m033_s03_hard_reads_*` proofs in `compiler/meshc/tests/e2e_m033_s03.rs`, rerun the named failing composed-read families, and only then do the full slice-level keep-list/verification closeout.

Resume notes:
1. Start in `mesher/storage/queries.mpl` around the current `get_event_neighbors(...)`, `evaluate_threshold_rule(...)`, `get_event_alert_rules(...)`, and `get_threshold_rules(...)` blocks; those are still in their pre-T04 state.
2. The build is green after the partial rewrites (`cargo run -q -p meshc -- build mesher` passed), so the file is at least syntactically consistent.
3. The last focused repro still showed these red families before the wrap-up: `e2e_m033_s03_composed_reads_joined_issue_and_team_rows`, `e2e_m033_s03_composed_reads_dashboard_aggregates`, `e2e_m033_s03_composed_reads_detail_and_issue_event_lists`, and `e2e_m033_s03_composed_reads_alert_lists_and_predicates`. Use those as the first resume targets instead of rerunning the full suite immediately.
4. No plan-invalidating blocker was found; this is unfinished execution, not a replan scenario.

## Verification

Reproduced the live S03 regression with `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture`, which failed in four named composed-read families: unresolved issue listing lost caller-visible fields, dashboard volume lost the bucket projection, `/api/v1/issues/:issue_id/events` crashed in the live handler, and the fresh new-issue alert never inserted. After stopping at the context-budget warning and fixing only the partial-pass syntax risks, reran `cargo run -q -p meshc -- build mesher`, which passed and confirmed the edited Mesh sources still compile. I did not rerun the failing test target after the partial rewrites, did not add the planned `hard_reads` proofs yet, and did not run the slice verifier script in this unit.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture` | 101 | ❌ fail | 180700ms |
| 2 | `cargo run -q -p meshc -- build mesher` | 0 | ✅ pass | 19600ms |


## Deviations

Stopped at the context-budget warning before finishing the planned `get_event_neighbors(...)` / `evaluate_threshold_rule(...)` rewrites, before adding `e2e_m033_s03_hard_reads_*`, and before rerunning the red composed-read proofs. This was a controlled wrap-up for clean handoff rather than a plan change.

## Known Issues

The T04 pass is incomplete. `get_event_neighbors(...)`, `evaluate_threshold_rule(...)`, `get_event_alert_rules(...)`, and `get_threshold_rules(...)` still need the remaining T04 work. The last reproduced test run still had four failing composed-read families: unresolved issue listing, dashboard aggregates, detail/event list routing, and alert insert/cooldown behavior. `scripts/verify-m033-s03.sh` and the planned `hard_reads` proofs were not added in this unit.

## Files Created/Modified

- `mesher/storage/queries.mpl`
