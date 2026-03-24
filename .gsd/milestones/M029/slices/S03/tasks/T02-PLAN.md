---
estimated_steps: 4
estimated_files: 6
skills_used:
  - debug-like-expert
  - lint
---

# T02: Rewrite API and service imports to canonical multiline form

**Slice:** S03 — Multiline imports and final formatter compliance
**Milestone:** M029

## Description

Finish the true hand-authored import cleanup surface in Mesher. This task converts the remaining five overlong `Storage.Queries` imports in the API and service layers to the exact `reference-backend/api/health.mpl` multiline shape so the later formatter waves are purely mechanical canonicalization rather than mixed style surgery.

## Steps

1. Read `reference-backend/api/health.mpl` again and reuse its exact multiline import structure instead of inventing a second style.
2. Rewrite the long imports in `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, and `mesher/api/team.mpl`, preserving imported names and order.
3. Rewrite the long imports in `mesher/services/project.mpl` and `mesher/services/user.mpl` to the same structure, again keeping names and order stable.
4. Verify that the five files no longer contain over-120-char `from` imports or spaced dotted paths, and stop if any change would require reopening formatter/compiler code.

## Must-Haves

- [ ] `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, and `mesher/api/team.mpl` use canonical parenthesized multiline imports.
- [ ] `mesher/services/project.mpl` and `mesher/services/user.mpl` use the same canonical multiline import style.
- [ ] This task stays inside Mesher source cleanup; it does not edit compiler files or `reference-backend/` source.

## Verification

- `! rg -n '^from .{121,}' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl`
- `! rg -n '^from .*\. ' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl`

## Observability Impact

- Signals added/changed: none; this task only normalizes import layout in five Mesher API/service files.
- How a future agent inspects this: compare each rewritten import against `reference-backend/api/health.mpl`, then rerun the two targeted `rg` checks across the five touched files.
- Failure state exposed: the overlong-import grep catches any missed single-line imports, while the spaced-dotted-path grep catches shape corruption such as `Storage. Queries` introduced during manual rewrites or later formatter churn.

## Inputs

- `reference-backend/api/health.mpl` — canonical multiline import style anchor
- `mesher/api/alerts.mpl` — long import to convert
- `mesher/api/dashboard.mpl` — long import to convert
- `mesher/api/team.mpl` — long import to convert
- `mesher/services/project.mpl` — long import to convert
- `mesher/services/user.mpl` — long import to convert

## Expected Output

- `mesher/api/alerts.mpl` — API import rewritten to canonical multiline form
- `mesher/api/dashboard.mpl` — API import rewritten to canonical multiline form
- `mesher/api/team.mpl` — API import rewritten to canonical multiline form
- `mesher/services/project.mpl` — service import rewritten to canonical multiline form
- `mesher/services/user.mpl` — service import rewritten to canonical multiline form
