# S03: Multiline imports and final formatter compliance

**Goal:** Convert Mesher's remaining overlong single-line imports to the canonical parenthesized multiline form, move every still-red Mesher source file onto the fixed formatter's canonical output, and close M029 with clean formatter/build proof on both dogfood apps.
**Demo:** `mesher/main.mpl`, `mesher/ingestion/routes.mpl`, `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/team.mpl`, `mesher/services/project.mpl`, and `mesher/services/user.mpl` use the `reference-backend/api/health.mpl` multiline import shape, `meshc fmt --check mesher` and `meshc fmt --check reference-backend` both pass, both builds pass, and `.gsd/milestones/M029/slices/S03/S03-UAT.md` records the final closeout evidence.

## Must-Haves

- R024 direct requirement: the 10 remaining Mesher `from ... import ...` lines longer than 120 chars are rewritten to parenthesized multiline imports with the opening `(` on the import line, one imported name per line, and the closing `)` on its own line.
- R024 direct requirement: every file currently red under `cargo run -q -p meshc -- fmt --check mesher` is rewritten to the fixed formatter's canonical output, and `cargo run -q -p meshc -- fmt --check mesher` returns clean.
- R011, R026, and R027 support: S03 stays in dogfood-source cleanup unless a formatter regression is actually reproduced, `reference-backend/` remains a regression-only proof target, and dotted-path grep plus build gates stay green on both apps.

## Verification

- `! rg -n '^from .{121,}' mesher -g '*.mpl'`
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- fmt --check reference-backend`
- `cargo run -q -p meshc -- build mesher`
- `cargo run -q -p meshc -- build reference-backend`
- `! rg -n '^from .*\. ' mesher reference-backend -g '*.mpl'`
- `(cargo run -q -p meshc -- fmt --check mesher > /tmp/m029-s03-fmt-mesher.log 2>&1 && test ! -s /tmp/m029-s03-fmt-mesher.log) || (rg -n 'error|panic|from .*\. ' /tmp/m029-s03-fmt-mesher.log && false)`
- `test -f .gsd/milestones/M029/slices/S03/S03-UAT.md`

## Observability / Diagnostics

- Runtime signals: none added; this slice is source-shape and formatter/build compliance work.
- Inspection surfaces: the long-import grep across `mesher/`, the spaced-dotted-path grep across `mesher/` and `reference-backend/`, `cargo run -q -p meshc -- fmt --check mesher`, `cargo run -q -p meshc -- fmt --check reference-backend`, both build commands, and `/tmp/m029-s03-fmt-mesher.log` for captured formatter diagnostics when the Mesher formatter gate fails.
- Failure visibility: the long-import grep exposes missed single-line imports, the dotted-path grep exposes formatter corruption like `Storage. Queries`, the captured `fmt --check` log preserves the first formatter/build failure signal for post-mortem inspection, and the build commands expose any syntax/type drift introduced while normalizing imports.
- Redaction constraints: no secrets should appear in these checks; keep evidence limited to repo paths, formatter output, and compiler diagnostics.

## Tasks

- [x] **T01: Rewrite entrypoint and ingestion imports to canonical multiline form** `est:25m`
  - Why: This closes the smallest manual-edit surface first and anchors S03 on the proven `reference-backend/api/health.mpl` import shape before any bulk formatter rewrites happen.
  - Files: `reference-backend/api/health.mpl`, `mesher/main.mpl`, `mesher/ingestion/routes.mpl`
  - Do: Rewrite the four overlong imports in `mesher/main.mpl` and the long `Storage.Queries` import in `mesher/ingestion/routes.mpl` to match the backend multiline anchor exactly, preserving imported names and order, and do not touch compiler code or `reference-backend/` source.
  - Verify: `! rg -n '^from .{121,}' mesher/main.mpl mesher/ingestion/routes.mpl && ! rg -n '^from .*\. ' mesher/main.mpl mesher/ingestion/routes.mpl`
  - Done when: Both files use the canonical parenthesized multiline style and neither file contains an over-120-char `from` import or spaced dotted path.
- [x] **T02: Rewrite API and service imports to canonical multiline form** `est:35m`
  - Why: These five files are the rest of the true human-authored readability surface; finishing them before formatter churn keeps the import intent reviewable and directly advances R024.
  - Files: `reference-backend/api/health.mpl`, `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/team.mpl`, `mesher/services/project.mpl`, `mesher/services/user.mpl`
  - Do: Convert the remaining five overlong `Storage.Queries` imports to the exact backend multiline pattern, keep imported names stable, and stop if any import rewrite would require compiler-side work rather than Mesher-source cleanup.
  - Verify: `! rg -n '^from .{121,}' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl && ! rg -n '^from .*\. ' mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl mesher/services/project.mpl mesher/services/user.mpl`
  - Done when: All five files use canonical multiline imports and there are no remaining long single-line or spaced dotted imports in that set.
- [ ] **T03: Canonicalize Mesher entrypoint and API modules with the fixed formatter** `est:35m`
  - Why: `mesher/main.mpl` and `mesher/api/*.mpl` are part of the known 35-file red surface, and formatting them as one scoped wave keeps the first bulk canonicalization pass under a single executor window.
  - Files: `mesher/main.mpl`, `mesher/api/alerts.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/detail.mpl`, `mesher/api/helpers.mpl`, `mesher/api/search.mpl`, `mesher/api/settings.mpl`, `mesher/api/team.mpl`
  - Do: Run the fixed formatter over `mesher/main.mpl` and `mesher/api/`, inspect the resulting diffs for unexpected multiline-import collapse or dotted-path corruption, and keep scope limited to formatter-authored canonicalization inside these eight files.
  - Verify: `cargo run -q -p meshc -- fmt mesher/main.mpl && cargo run -q -p meshc -- fmt mesher/api && cargo run -q -p meshc -- fmt --check mesher/main.mpl && cargo run -q -p meshc -- fmt --check mesher/api && ! rg -n '^from .{121,}' mesher/main.mpl mesher/api/alerts.mpl mesher/api/dashboard.mpl mesher/api/team.mpl && ! rg -n '^from .*\. ' mesher/main.mpl mesher/api -g '*.mpl'`
  - Done when: The eight files are on canonical formatter output, the multiline imports in `main.mpl`, `alerts.mpl`, `dashboard.mpl`, and `team.mpl` stay multiline, and no spaced dotted imports appear.
- [ ] **T04: Canonicalize Mesher ingestion and storage modules with the fixed formatter** `est:35m`
  - Why: The ingestion/storage wave carries the `routes.mpl` multiline import plus the storage files that S02 intentionally left for mechanical formatter cleanup, so it is the next bounded canonicalization unit.
  - Files: `mesher/ingestion/auth.mpl`, `mesher/ingestion/fingerprint.mpl`, `mesher/ingestion/pipeline.mpl`, `mesher/ingestion/routes.mpl`, `mesher/ingestion/validation.mpl`, `mesher/ingestion/ws_handler.mpl`, `mesher/storage/queries.mpl`, `mesher/storage/schema.mpl`, `mesher/storage/writer.mpl`
  - Do: Run the fixed formatter over `mesher/ingestion/` and `mesher/storage/`, accept only mechanical canonicalization in those files, and treat any dotted-path or multiline-import regression as a stop signal rather than expanding back into compiler work.
  - Verify: `cargo run -q -p meshc -- fmt mesher/ingestion && cargo run -q -p meshc -- fmt mesher/storage && cargo run -q -p meshc -- fmt --check mesher/ingestion && cargo run -q -p meshc -- fmt --check mesher/storage && ! rg -n '^from .{121,}' mesher/ingestion/routes.mpl && ! rg -n '^from .*\. ' mesher/ingestion mesher/storage -g '*.mpl'`
  - Done when: The nine files round-trip cleanly through the formatter wave, `routes.mpl` stays multiline, and no spaced dotted imports appear in ingestion or storage modules.
- [ ] **T05: Canonicalize Mesher service modules with the fixed formatter** `est:30m`
  - Why: `mesher/services/` is its own eight-file formatter-red cluster, and splitting it out keeps the bulk rewrite reviewable while preserving the multiline imports added to `project.mpl` and `user.mpl`.
  - Files: `mesher/services/event_processor.mpl`, `mesher/services/org.mpl`, `mesher/services/project.mpl`, `mesher/services/rate_limiter.mpl`, `mesher/services/retention.mpl`, `mesher/services/stream_manager.mpl`, `mesher/services/user.mpl`, `mesher/services/writer.mpl`
  - Do: Run the fixed formatter over `mesher/services/`, accept the canonicalization rewrite for those eight files, and confirm the long imports in `project.mpl` and `user.mpl` are still parenthesized multiline after formatting.
  - Verify: `cargo run -q -p meshc -- fmt mesher/services && cargo run -q -p meshc -- fmt --check mesher/services && ! rg -n '^from .{121,}' mesher/services/project.mpl mesher/services/user.mpl && ! rg -n '^from .*\. ' mesher/services -g '*.mpl'`
  - Done when: All service files are formatter-clean, both rewritten service imports stay multiline, and no spaced dotted imports appear anywhere in `mesher/services/`.
- [ ] **T06: Canonicalize remaining Mesher files and record final slice proof** `est:45m`
  - Why: The remaining types/tests/migrations files finish the red `fmt --check` surface, and this last task owns the truthful closeout gate across Mesher plus regression-proof `reference-backend/`.
  - Files: `mesher/migrations/20260216120000_create_initial_schema.mpl`, `mesher/migrations/20260226000000_seed_default_org.mpl`, `mesher/tests/fingerprint.test.mpl`, `mesher/tests/validation.test.mpl`, `mesher/types/alert.mpl`, `mesher/types/event.mpl`, `mesher/types/issue.mpl`, `mesher/types/project.mpl`, `mesher/types/retention.mpl`, `mesher/types/user.mpl`, `.gsd/milestones/M029/slices/S03/S03-UAT.md`
  - Do: Run the fixed formatter over the remaining Mesher files, execute the full slice closeout gate on `mesher/` and `reference-backend/`, and write `S03-UAT.md` with the passed command set and any residual watchouts for future milestone closure.
  - Verify: `cargo run -q -p meshc -- fmt mesher/types && cargo run -q -p meshc -- fmt mesher/tests && cargo run -q -p meshc -- fmt mesher/migrations && cargo run -q -p meshc -- fmt --check mesher && cargo run -q -p meshc -- fmt --check reference-backend && cargo run -q -p meshc -- build mesher && cargo run -q -p meshc -- build reference-backend && ! rg -n '^from .{121,}' mesher -g '*.mpl' && ! rg -n '^from .*\. ' mesher reference-backend -g '*.mpl' && test -f .gsd/milestones/M029/slices/S03/S03-UAT.md`
  - Done when: The remaining Mesher files are formatter-clean, both apps pass formatter/build gates, repo-wide long single-line and spaced dotted import greps are clean, and `S03-UAT.md` records the green closeout evidence.

## Files Likely Touched

- `mesher/main.mpl`
- `mesher/api/alerts.mpl`
- `mesher/api/dashboard.mpl`
- `mesher/api/detail.mpl`
- `mesher/api/helpers.mpl`
- `mesher/api/search.mpl`
- `mesher/api/settings.mpl`
- `mesher/api/team.mpl`
- `mesher/ingestion/auth.mpl`
- `mesher/ingestion/fingerprint.mpl`
- `mesher/ingestion/pipeline.mpl`
- `mesher/ingestion/routes.mpl`
- `mesher/ingestion/validation.mpl`
- `mesher/ingestion/ws_handler.mpl`
- `mesher/services/event_processor.mpl`
- `mesher/services/org.mpl`
- `mesher/services/project.mpl`
- `mesher/services/rate_limiter.mpl`
- `mesher/services/retention.mpl`
- `mesher/services/stream_manager.mpl`
- `mesher/services/user.mpl`
- `mesher/services/writer.mpl`
- `mesher/storage/queries.mpl`
- `mesher/storage/schema.mpl`
- `mesher/storage/writer.mpl`
- `mesher/tests/fingerprint.test.mpl`
- `mesher/tests/validation.test.mpl`
- `mesher/types/alert.mpl`
- `mesher/types/event.mpl`
- `mesher/types/issue.mpl`
- `mesher/types/project.mpl`
- `mesher/types/retention.mpl`
- `mesher/types/user.mpl`
- `mesher/migrations/20260216120000_create_initial_schema.mpl`
- `mesher/migrations/20260226000000_seed_default_org.mpl`
- `.gsd/milestones/M029/slices/S03/S03-UAT.md`
