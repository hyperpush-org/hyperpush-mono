---
title: Production Backend Proof
description: Compact public-secondary handoff for Mesh's starter/examples-first backend story, Mesher maintainer runbook, and retained backend-only proof verifiers
prev: false
next: false
---

# Production Backend Proof

This is the compact public-secondary handoff for Mesh's backend proof story.

Public readers should still stay scaffold/examples first: start with [Clustered Example](/docs/getting-started/clustered-example/), [`examples/todo-sqlite/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md), or [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) before using this page as a deeper handoff.

This page only names the deeper maintainer surfaces behind that public story: Mesher as the maintained app, and a retained backend-only verifier kept behind a named replay instead of a public repo-root runbook.

## Canonical surfaces

- [Clustered Example](/docs/getting-started/clustered-example/) — public scaffold-first clustered walkthrough
- [`examples/todo-sqlite/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) — honest local single-node starter
- [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) — shared/deployable PostgreSQL starter
- [`mesher/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md) — deeper maintained app runbook for repo maintainers
- `bash scripts/verify-m051-s01.sh` — maintainer verifier for the shipped Mesher app surface
- `bash scripts/verify-m051-s02.sh` — maintainer verifier for the retained backend-only proof replay
- `bash reference-backend/scripts/verify-production-proof-surface.sh` — compatibility verifier that keeps this public proof page truthful

## Named maintainer verifiers

These are the named commands behind the current deeper backend-maintainer story:

```bash
bash scripts/verify-m051-s01.sh
bash scripts/verify-m051-s02.sh
bash reference-backend/scripts/verify-production-proof-surface.sh
```

Use `bash scripts/verify-m051-s01.sh` when you are verifying the maintained deeper app that repo maintainers actually work on. Use `bash scripts/verify-m051-s02.sh` when you need the retained backend-only proof replay without turning its internal fixture layout into public teaching.

## Retained backend-only recovery signals

When `bash scripts/verify-m051-s02.sh` fails on the retained backend-only proof, inspect the recovery markers it preserves:

- `restart_count`
- `last_exit_reason`
- `recovered_jobs`
- `last_recovery_at`
- `last_recovery_job_id`
- `last_recovery_count`
- `recovery_active`

Those signals stay maintainer-facing on purpose: they prove the retained backend-only recovery story without turning the compatibility fixture tree into a public tutorial.

## When to use this page vs the generic guides

Use [Web](/docs/web/), [Databases](/docs/databases/), [Testing](/docs/testing/), [Concurrency](/docs/concurrency/), or [Developer Tools](/docs/tooling/) when you want a subsystem API in isolation.

Use this page when you need the handoff from the public starter/examples-first route into the maintainer-only backend surfaces. From here, continue to [`mesher/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md) plus `bash scripts/verify-m051-s01.sh` for the deeper maintained app, or use `bash scripts/verify-m051-s02.sh` for the retained backend-only proof.

## Failure inspection map

If a maintainer proof fails, rerun the smallest named surface that matches the drift:

- **Public proof-page contract:** `bash reference-backend/scripts/verify-production-proof-surface.sh`
- **Maintained deeper app:** [`mesher/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md) and `bash scripts/verify-m051-s01.sh`
- **Retained backend-only proof:** `bash scripts/verify-m051-s02.sh`

If a public docs page starts teaching the old repo-root compatibility surfaces again, treat that as contract drift instead of a docs cleanup detail.
