# M051: Mesher as the Living Reference App

## Vision
Replace `reference-backend/` role-by-role so Mesher becomes the maintained internal reference app on current Mesh patterns while public guidance stays scaffold/examples-first and no surviving proof chain depends on the retired app.

## Slice Overview
| ID | Slice | Risk | Depends | Done | After this |
|----|-------|------|---------|------|------------|
| S01 | Modernize Mesher bootstrap and maintainer run path | high | — | ✅ | A maintainer can follow the Mesher runbook to migrate, build, and run Mesher against Postgres on the current runtime contract, and the live proof rails show readiness through Mesher’s real app surface. |
| S02 | Extract retained backend-only proof out of reference-backend | high | S01 | ✅ | Maintainers can still replay the backend-specific deploy, recovery, and health-style proof that matters after retirement, but it now lives in retained harnesses or maintainer material instead of a public top-level app. |
| S03 | Migrate tooling and editor rails to a bounded backend fixture | medium | S02 | ✅ | LSP, tooling, formatter, and editor smoke rails replay against a small retained backend-shaped fixture instead of `reference-backend/`, preserving the bounded project semantics those rails actually need. |
| S04 | Retarget public docs, scaffold, and skills to the examples-first story | medium | S01, S02, S03 | ✅ | A public reader following README, VitePress docs, scaffold output, or bundled skill guidance lands on scaffold output and `/examples`, while Mesher is described only as the deeper maintained app for repo maintainers. |
| S05 | Delete reference-backend and close the assembled acceptance rail | medium | S02, S03, S04 | ✅ | The repo ships without `reference-backend/`, and the final acceptance bundle proves Mesher live runtime, retained backend proof, migrated tooling rails, and examples-first docs together on the post-deletion tree. |
