---
estimated_steps: 4
estimated_files: 8
skills_used:
  - lint
  - debug-like-expert
---

# T03: Canonicalize Mesher entrypoint and API modules with the fixed formatter

**Slice:** S03 — Multiline imports and final formatter compliance
**Milestone:** M029

## Description

Accept the first bounded formatter-canonicalization wave now that the hand-edited imports are in place. This task runs the fixed formatter over `mesher/main.mpl` and `mesher/api/*.mpl`, verifies the rewritten imports stay multiline, and treats any dotted-path corruption or multiline collapse as a regression to stop on instead of papering over it with more source edits.

## Steps

1. Read the current contents of `mesher/main.mpl` and the seven `mesher/api/*.mpl` files so the formatter rewrite is reviewed against real source, not treated as blind churn.
2. Run `cargo run -q -p meshc -- fmt mesher/main.mpl` and `cargo run -q -p meshc -- fmt mesher/api` to move this eight-file group onto canonical formatter output.
3. Inspect the resulting diffs, with special attention to the multiline imports in `mesher/main.mpl`, `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, and `mesher/api/team.mpl` plus any dotted module paths.
4. Verify the group is formatter-clean and that no long single-line or spaced dotted imports reappeared in these files.

## Must-Haves

- [ ] `mesher/main.mpl` and all seven `mesher/api/*.mpl` files are on the fixed formatter's canonical output.
- [ ] The multiline imports in `mesher/main.mpl`, `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, and `mesher/api/team.mpl` remain multiline after formatting.
- [ ] No new compiler work is started unless a real dotted-path or multiline-import regression is reproduced in this wave.

## Verification

- `cargo run -q -p meshc -- fmt mesher/main.mpl && cargo run -q -p meshc -- fmt mesher/api && cargo run -q -p meshc -- fmt --check mesher/main.mpl && cargo run -q -p meshc -- fmt --check mesher/api`
- `! rg -n '^from .{121,}' mesher/main.mpl mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl && ! rg -n '^from .*\. ' mesher/main.mpl mesher/api -g '*.mpl'`

## Observability Impact

- Signals added/changed: none; this task only moves `mesher/main.mpl` and `mesher/api/*.mpl` onto canonical formatter output.
- How a future agent inspects this: rerun the scoped formatter wave and `fmt --check` commands for `mesher/main.mpl` and `mesher/api`, then inspect the rewritten imports in `mesher/main.mpl`, `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, and `mesher/api/team.mpl` plus the spaced-dotted-path grep across `mesher/api`.
- Failure state exposed: `fmt --check` shows whether these eight files are still off canonical output, the long-import grep catches multiline collapse back to overlong single-line imports, and the spaced-dotted-path grep catches formatter corruption such as `Api. Alerts` or `Storage. Queries` even if the formatter output is otherwise idempotent.

## Inputs

- `mesher/main.mpl` — T01 multiline imports plus formatter-red entrypoint file
- `mesher/api/alerts.mpl` — T02 multiline import plus formatter-red API file
- `mesher/api/dashboard.mpl` — T02 multiline import plus formatter-red API file
- `mesher/api/detail.mpl` — formatter-red API file
- `mesher/api/helpers.mpl` — formatter-red API file
- `mesher/api/search.mpl` — formatter-red API file
- `mesher/api/settings.mpl` — formatter-red API file
- `mesher/api/team.mpl` — T02 multiline import plus formatter-red API file

## Expected Output

- `mesher/main.mpl` — canonical formatter output for the entrypoint file
- `mesher/api/alerts.mpl` — canonical formatter output for the API module
- `mesher/api/dashboard.mpl` — canonical formatter output for the API module
- `mesher/api/detail.mpl` — canonical formatter output for the API module
- `mesher/api/helpers.mpl` — canonical formatter output for the API module
- `mesher/api/search.mpl` — canonical formatter output for the API module
- `mesher/api/settings.mpl` — canonical formatter output for the API module
- `mesher/api/team.mpl` — canonical formatter output for the API module
