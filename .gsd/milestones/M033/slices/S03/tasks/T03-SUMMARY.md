---
id: T03
parent: S03
milestone: M033
key_files:
  - compiler/meshc/tests/e2e_m033_s03.rs
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
key_decisions:
  - Use the live Mesher HTTP/API harness for S03 composed-read proofs instead of extending the copied storage-only Mesh probe, because the probe failures were no longer an honest signal for caller-contract correctness.
  - Treat the new failures as product-side caller-contract bugs in Mesher routes/alert flow rather than as more storage-probe drift; the next unit should debug route serialization and live handler behavior before changing the Rust harness again.
duration: ""
verification_result: mixed
completed_at: 2026-03-25T19:54:34.485Z
blocker_discovered: true
---

# T03: Rewrite the S03 composed-read harness onto live Mesher routes and capture the remaining caller-contract regressions as the blocker

**Rewrite the S03 composed-read harness onto live Mesher routes and capture the remaining caller-contract regressions as the blocker**

## What Happened

I replaced the failing storage-probe composed-read tests in `compiler/meshc/tests/e2e_m033_s03.rs` with a live Mesher harness built around the real Mesher binary, HTTP requests, and direct Postgres assertions. The new harness now spawns Mesher on ephemeral ports, waits for the settings route to go healthy, drives live issue/dashboard/detail/alert/team endpoints, and captures Mesher stdout/stderr on failure so the proof surface is aligned with the real caller contracts instead of the broken copied storage probe.

After that rewrite, I reran the task gate (`cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture`). The old storage-probe failures disappeared, but the live Mesher surface exposed four real contract problems instead: the issue list route returned blank identity/label fields, the dashboard volume route returned counts with missing buckets, the issue-events route crashed with a Mesh `non-exhaustive match in switch`, and the alert-flow proof never observed the expected fresh new-issue alert. Those are no longer probe artifacts; they are product-side route/handler regressions in the current Mesher path. I recorded the resume note in `.gsd/KNOWLEDGE.md` and saved decision D062 so the next unit starts from the live-route failure surfaces rather than rediscovering the obsolete storage-probe blocker.

## Verification

Ran the authoritative T03 gate after the live-harness rewrite: `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture`. The test target compiled and exercised the new Mesher-backed harness, then failed on live caller-contract regressions rather than on the old storage-probe staging bugs. I did not start another verification cycle or rerun `cargo run -q -p meshc -- build mesher` after the wrap-up warning; the next unit should fix the live route/alert failures first, then rerun the composed-read gate and the build/script checks.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture` | 101 | ❌ fail | 107000ms |


## Deviations

I did not finish the planned Mesher-backed proof surface closeout. I rewrote the composed-read harness onto live Mesher HTTP routes, but the first authoritative rerun surfaced real caller-contract regressions in Mesher itself, so I stopped in wrap-up mode instead of attempting another debugging round without enough context budget.

## Known Issues

The Mesher-backed composed-read gate still fails. `/api/v1/projects/:project_id/issues` currently returns blank `id`/`title`/`level`/`status` fields while counts/timestamps survive, `/api/v1/projects/:project_id/dashboard/volume` currently drops `bucket` values while keeping counts, `/api/v1/issues/:issue_id/events` currently crashes in `_handle_list_issue_events` with `non-exhaustive match in switch`, and the new-issue ingest path did not create the expected fresh alert for the live alert proof.

## Files Created/Modified

- `compiler/meshc/tests/e2e_m033_s03.rs`
- `.gsd/KNOWLEDGE.md`
- `.gsd/DECISIONS.md`
