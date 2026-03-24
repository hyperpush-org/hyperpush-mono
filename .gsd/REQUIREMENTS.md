# Requirements

This file is the explicit capability and coverage contract for the project.

## Active

### R007 — Mesh projects have a believable dependency/package workflow for building and shipping backend applications with reproducible inputs.
- Class: launchability
- Status: active
- Description: Mesh projects have a believable dependency/package workflow for building and shipping backend applications with reproducible inputs.
- Why it matters: A language may have good runtime features and still fail as a serious backend option if dependency flow is rough or confidence-eroding.
- Source: inferred
- Primary owning slice: M030/S01 (provisional)
- Supporting slices: M030/S02 (provisional)
- Validation: mapped
- Notes: This sits after the M028 trust baseline but is already part of the capability contract.

### R008 — Mesh documentation and examples show a production-style backend path and do not rely mainly on toy examples to make the language look ready.
- Class: launchability
- Status: active
- Description: Mesh documentation and examples show a production-style backend path and do not rely mainly on toy examples to make the language look ready.
- Why it matters: The user explicitly said “docs/examples don’t prove real use” would be a failure.
- Source: user
- Primary owning slice: M028/S06
- Supporting slices: M028/S01, M028/S03, M028/S04, M028/S05
- Validation: mapped
- Notes: S03 advanced the documentation truth surface by syncing README, website tooling/testing/cheatsheet docs, reference-backend docs, and the VS Code README to the verified `meshc fmt`, project-directory `meshc test`, honest `--coverage` contract, and JSON-RPC-proven LSP feature set. S06 added the production proof page, README/generic-doc routing, and doc-truth verifier, and S07 restored the green recovery-aware backend proofs. R008 remains open because S08 still needs to reconcile the public README/docs/UAT promotion surfaces so they point only at those now-green recovery-aware proof paths.

### R010 — The project can point to specific ways Mesh is easier to deploy, measurably fast, and nicer for backend development rather than vaguely claiming it is “better than Elixir.”
- Class: differentiator
- Status: active
- Description: The project can point to specific ways Mesh is easier to deploy, measurably fast, and nicer for backend development rather than vaguely claiming it is “better than Elixir.”
- Why it matters: The user’s comparison target is clear, but the comparison needs to be grounded in measurable strengths instead of ecosystem parity rhetoric.
- Source: user
- Primary owning slice: M029/S01 (provisional)
- Supporting slices: M028/S04, M028/S06, M029/S02 (provisional)
- Validation: mapped
- Notes: M028 establishes the baseline proof needed before these comparisons are sharpened further.

### R011 — New language/runtime work after M028 should come from real backend friction discovered while using Mesh for actual backend code.
- Class: differentiator
- Status: active
- Description: New language/runtime work after M028 should come from real backend friction discovered while using Mesh for actual backend code.
- Why it matters: This keeps the project from chasing clever language features that do not improve the target use case.
- Source: user
- Primary owning slice: M029/S02 (provisional)
- Supporting slices: M029/S03 (provisional)
- Validation: mapped
- Notes: This is the guardrail against speculative feature churn.

### R012 — After the canonical API + DB + migrations + jobs path is proven, Mesh continues toward the broader backend space the user wants: long-running supervised services, realtime systems, and distributed backends.
- Class: core-capability
- Status: active
- Description: After the canonical API + DB + migrations + jobs path is proven, Mesh continues toward the broader backend space the user wants: long-running supervised services, realtime systems, and distributed backends.
- Why it matters: The long-term vision is “all types of backend code,” not only one app shape.
- Source: user
- Primary owning slice: M031/S01 (provisional)
- Supporting slices: M031/S02 (provisional), M031/S03 (provisional)
- Validation: mapped
- Notes: This remains in scope for the project, but it follows the first credibility milestone.

### R013 — A blocking Mesh language/runtime/tooling limitation is not worked around indefinitely; it is fixed in Mesh and then used in this app.
- Class: constraint
- Status: active
- Description: A blocking Mesh language/runtime/tooling limitation is not worked around indefinitely; it is fixed in Mesh and then used in this app.
- Why it matters: This app is also a Mesh dogfooding vehicle.
- Source: user
- Primary owning slice: M023/S01
- Supporting slices: M027/S01 (provisional)
- Validation: mapped
- Notes: User instruction was explicit: stop and fix the language if a real limitation blocks the app.

### R014 — The first product loop focuses on existing creator tokens joining the fund before adding token-launch convenience.
- Class: constraint
- Status: active
- Description: The first product loop focuses on existing creator tokens joining the fund before adding token-launch convenience.
- Why it matters: This reduces first-proof complexity while still proving the real product thesis.
- Source: inferred
- Primary owning slice: M023/S02
- Supporting slices: M023/S06
- Validation: mapped
- Notes: Launch-through-app remains desirable if Bags support proves smooth enough later.

## Validated

### R001 — Mesh has an explicit definition of what “production ready language needs to have” means for this repo, and that baseline can be checked through concrete proof rather than vague claims.
- Class: launchability
- Status: validated
- Description: Mesh has an explicit definition of what “production ready language needs to have” means for this repo, and that baseline can be checked through concrete proof rather than vague claims.
- Why it matters: Without a baseline contract, the work turns into an endless feature list and nobody can tell whether Mesh actually became more trustworthy.
- Source: inferred
- Primary owning slice: M028/S01
- Supporting slices: M028/S06
- Validation: Validated by M028/S01 through the shipped `reference-backend/` package, canonical startup contract (`DATABASE_URL`, `PORT`, `JOB_POLL_MS`), package README/.env example, compiler e2e proof (`e2e_reference_backend_builds`, `e2e_reference_backend_runtime_starts`, `e2e_reference_backend_postgres_smoke`), migration status/up commands, and the package-local smoke path proving the baseline with concrete commands instead of abstract claims.
- Notes: S01 established the repo’s first concrete backend trust baseline around one auditable API + DB + migrations + jobs workflow.

### R002 — Mesh can power a real backend shape with an HTTP API, persistent database state, migrations, and background jobs in one coherent flow.
- Class: core-capability
- Status: validated
- Description: Mesh can power a real backend shape with an HTTP API, persistent database state, migrations, and background jobs in one coherent flow.
- Why it matters: This is the first serious proof target for trusting Mesh for a real production app backend in any capacity.
- Source: user
- Primary owning slice: M028/S01
- Supporting slices: M028/S02, M028/S04, M028/S05, M028/S06
- Validation: Validated by M028/S01 through live end-to-end verification of `reference-backend/`: compiler/runtime build, explicit missing-env failure, Postgres-backed startup, migration status and apply, `GET /health`, `POST /jobs`, `GET /jobs/:id`, timer-driven worker transition from `pending` to `processed`, package-local `reference-backend/scripts/smoke.sh`, and compiler-facing ignored smoke coverage in `e2e_reference_backend_postgres_smoke`.
- Notes: S01 closed the first real backend proof path by wiring API, DB, migrations, and background jobs into one auditable package and by fixing Mesh runtime issues instead of leaving them as app-level workarounds.

### R003 — The runtime path behind the canonical backend flow is exercised by automated verification strongly enough that the path is not just “implemented,” but trusted.
- Class: quality-attribute
- Status: validated
- Description: The runtime path behind the canonical backend flow is exercised by automated verification strongly enough that the path is not just “implemented,” but trusted.
- Why it matters: A backend language loses credibility quickly if its basic runtime surfaces only work in isolated or manual scenarios.
- Source: inferred
- Primary owning slice: M028/S02
- Supporting slices: M028/S06
- Validation: Validated by M028/S02 through live Postgres-backed compiler e2e coverage in `compiler/meshc/tests/e2e_reference_backend.rs`: `e2e_reference_backend_runtime_starts`, `e2e_reference_backend_migration_status_and_apply`, `e2e_reference_backend_job_flow_updates_health_and_db`, `e2e_reference_backend_claim_contention_is_not_failure`, and `e2e_reference_backend_multi_instance_claims_once` proving migration pending→applied truth, HTTP/DB/health agreement for job lifecycle state, and two-instance exact-once shared-DB processing without benign claim contention inflating `failed_jobs` or `last_error`.
- Notes: S02 kept runtime-correctness proof on the canonical reference-backend harness and moved exact-once truth to direct `jobs` reads, cross-instance `/jobs/:id`, and per-instance processed-job logs while treating `/health.failed_jobs` + `/health.last_error` as the stable contention signal.

### R004 — Mesh concurrency and supervision are proven under crash, restart, and failure-reporting scenarios instead of only being advertised as features.
- Class: quality-attribute
- Status: validated
- Description: Mesh concurrency and supervision are proven under crash, restart, and failure-reporting scenarios instead of only being advertised as features.
- Why it matters: The user explicitly said “concurrency exists but isn’t trustworthy” would be a failure state for the project.
- Source: user
- Primary owning slice: M028/S05
- Supporting slices: M028/S02, M028/S06, M028/S07
- Validation: Validated by M028/S07 through live recovery proof on `reference-backend/`: `cargo run -p meshc -- build reference-backend`, `cargo run -p meshc -- fmt --check reference-backend`, `cargo run -p meshc -- test reference-backend`, and ignored compiler e2e proofs `e2e_reference_backend_worker_crash_recovers_job`, `e2e_reference_backend_worker_restart_is_visible_in_health`, `e2e_reference_backend_process_restart_recovers_inflight_job`, `e2e_reference_backend_migration_status_and_apply`, and `e2e_reference_backend_deploy_artifact_smoke`. The proof shows a real degraded/recovering window, restart-count and exit metadata on `/health`, stale-cutoff reclaim of abandoned `processing` rows, and exact-once completion after both worker crash and whole-process restart on the canonical backend path.
- Notes: S07 closed the remaining concurrency-trust gap by moving recovery ownership onto the restarted worker boundary, exposing restart evidence on `/health`, and proving crash/process restart recovery against the same reference backend used for the rest of M028.

### R009 — Mesh proves itself through a real reference backend that exercises the language as a backend platform instead of proving subsystems only in isolation.
- Class: differentiator
- Status: validated
- Description: Mesh proves itself through a real reference backend that exercises the language as a backend platform instead of proving subsystems only in isolation.
- Why it matters: Dogfooding is how the project turns “all types of backend code” from ambition into grounded engineering pressure.
- Source: inferred
- Primary owning slice: M028/S06
- Supporting slices: M028/S01, M028/S02, M028/S05, M028/S07
- Validation: Validated by M028/S07 on top of the existing S01-S06 proof surface. The `reference-backend/` package now has green end-to-end build/fmt/test, migration/deploy smoke, worker-crash recovery, restart-visibility, and whole-process restart proofs in `compiler/meshc/tests/e2e_reference_backend.rs`, so Mesh is being judged through one real backend that exercises compiler, runtime, HTTP, Postgres, migrations, background jobs, and recovery instead of isolated subsystem demos.
- Notes: The reference backend is now a genuinely recovery-aware end-to-end proof target. S08 still needs to reconcile README/docs/UAT promotion to point only at the green recovery-aware paths, but the backend proof itself is now validated.

### R005 — Mesh’s native-binary workflow is proven through a deployment path that feels closer to shipping a Go app than to assembling a fragile language stack.
- Class: launchability
- Status: validated
- Description: Mesh’s native-binary workflow is proven through a deployment path that feels closer to shipping a Go app than to assembling a fragile language stack.
- Why it matters: Easier deployment is one of the first ways the user wants Mesh to beat Elixir.
- Source: user
- Primary owning slice: M028/S04
- Supporting slices: M028/S06
- Validation: Validated by M028/S04 through live native-deployment proof for `reference-backend/`: `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture`, staged bundle verification via `reference-backend/scripts/stage-deploy.sh`, `cargo test -p meshc e2e_self_contained_binary -- --nocapture`, ignored operational proof `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`, missing-artifact diagnostics in `apply-deploy-migrations.sh`, and operator-facing docs/env contract in `reference-backend/README.md` and `reference-backend/.env.example`. Proof covers build-host staging, runtime-host `psql` migration apply without `meshc`, staged binary startup outside the repo root, `/health`, job creation + processing, `_mesh_migrations` recording, and log redaction of `DATABASE_URL`.
- Notes: S04 proves one honest boring deployment path for `reference-backend/` with staged native binary + checked-in SQL artifact + thin runtime scripts. It does not claim broader platform packaging or supervision/recovery guarantees; those remain for later slices.

### R006 — Diagnostics, formatter, LSP, tests, and the coverage story are credible enough that a backend engineer can use Mesh daily without fighting the toolchain.
- Class: quality-attribute
- Status: validated
- Description: Diagnostics, formatter, LSP, tests, and the coverage story are credible enough that a backend engineer can use Mesh daily without fighting the toolchain.
- Why it matters: Better DX is part of the explicit comparison target against Elixir, and weak tooling would erase gains from static typing or native binaries.
- Source: user
- Primary owning slice: M028/S03
- Supporting slices: M030/S01 (provisional), M030/S02 (provisional)
- Validation: S03 closure reran the full tooling trust gate on `reference-backend/`: `cargo test -p mesh-fmt -- --nocapture`, `cargo test -p meshc --test e2e_fmt -- --nocapture`, `cargo run -p meshc -- fmt --check reference-backend`, `cargo run -p meshc -- test reference-backend`, `! cargo run -p meshc -- test --coverage reference-backend`, `cargo test -p meshc --test tooling_e2e -- --nocapture`, `cargo test -p meshc --test e2e_lsp -- --nocapture`, `cargo test -p mesh-lsp -- --nocapture`, and the stale-string sweep over README/website/VS Code/reference-backend docs all passed.
- Notes: The toolchain should be judged against the real reference backend, not only tiny fixtures.

## Deferred

### R020 — Mesh eventually offers a stronger debugger/profiler/trace surface suitable for deeper production diagnostics.
- Class: operability
- Status: deferred
- Description: Mesh eventually offers a stronger debugger/profiler/trace surface suitable for deeper production diagnostics.
- Why it matters: Mature backend ecosystems are judged heavily on observability and debugging, but this should not swallow the first trust milestone.
- Source: research
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred until the canonical backend path and boring deploy story are proven.

### R021 — Registry, publishing flow, package trust, and ecosystem polish rise from “credible enough” to “mature ecosystem experience.”
- Class: admin/support
- Status: deferred
- Description: Registry, publishing flow, package trust, and ecosystem polish rise from “credible enough” to “mature ecosystem experience.”
- Why it matters: It matters for adoption, but the first milestone should not stall on ecosystem breadth.
- Source: research
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: The baseline package flow is active scope; broad ecosystem polish is later.

### R022 — Operators get richer admin controls, manual retries, and deeper operational tooling.
- Class: operability
- Status: deferred
- Description: Operators get richer admin controls, manual retries, and deeper operational tooling.
- Why it matters: It helps long-term operability once the core loop is proven.
- Source: inferred
- Primary owning slice: M027/S02 (provisional)
- Supporting slices: none
- Validation: unmapped
- Notes: Day-one requirement is visible failure, not a full operator cockpit.

## Out of Scope

### R030 — The project is not being planned primarily as a frontend-first language effort.
- Class: anti-feature
- Status: out-of-scope
- Description: The project is not being planned primarily as a frontend-first language effort.
- Why it matters: This prevents scope confusion and preserves the explicit server/backend bias from the discussion.
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Mesh remains general-purpose, but planning and proof are backend-led.

### R031 — M028 should not become a broad syntax/features sprint before the backend trust baseline is proven.
- Class: anti-feature
- Status: out-of-scope
- Description: M028 should not become a broad syntax/features sprint before the backend trust baseline is proven.
- Why it matters: This keeps the first milestone honest and evidence-first.
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Feature expansion belongs after the hardening milestone unless a blocker is found on the golden path.

### R032 — The project will not call Mesh production-ready based only on feature lists, benchmarks, or toy examples.
- Class: constraint
- Status: out-of-scope
- Description: The project will not call Mesh production-ready based only on feature lists, benchmarks, or toy examples.
- Why it matters: This prevents the exact kind of weak proof the user rejected.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Honest proof is a non-negotiable scope boundary.

### R033 — This build does not treat a native mobile app as a first-class deliverable.
- Class: constraint
- Status: out-of-scope
- Description: This build does not treat a native mobile app as a first-class deliverable.
- Why it matters: It keeps attention on the web product and Mesh backend.
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Web is the primary surface for the planned milestones.

## Traceability

| ID | Class | Status | Primary owner | Supporting | Proof |
|---|---|---|---|---|---|
| R001 | launchability | validated | M028/S01 | M028/S06 | Validated by M028/S01 through the shipped `reference-backend/` package, canonical startup contract (`DATABASE_URL`, `PORT`, `JOB_POLL_MS`), package README/.env example, compiler e2e proof (`e2e_reference_backend_builds`, `e2e_reference_backend_runtime_starts`, `e2e_reference_backend_postgres_smoke`), migration status/up commands, and the package-local smoke path proving the baseline with concrete commands instead of abstract claims. |
| R002 | core-capability | validated | M028/S01 | M028/S02, M028/S04, M028/S05, M028/S06 | Validated by M028/S01 through live end-to-end verification of `reference-backend/`: compiler/runtime build, explicit missing-env failure, Postgres-backed startup, migration status and apply, `GET /health`, `POST /jobs`, `GET /jobs/:id`, timer-driven worker transition from `pending` to `processed`, package-local `reference-backend/scripts/smoke.sh`, and compiler-facing ignored smoke coverage in `e2e_reference_backend_postgres_smoke`. |
| R003 | quality-attribute | validated | M028/S02 | M028/S06 | Validated by M028/S02 through live Postgres-backed compiler e2e coverage in `compiler/meshc/tests/e2e_reference_backend.rs`: `e2e_reference_backend_runtime_starts`, `e2e_reference_backend_migration_status_and_apply`, `e2e_reference_backend_job_flow_updates_health_and_db`, `e2e_reference_backend_claim_contention_is_not_failure`, and `e2e_reference_backend_multi_instance_claims_once` proving migration pending→applied truth, HTTP/DB/health agreement for job lifecycle state, and two-instance exact-once shared-DB processing without benign claim contention inflating `failed_jobs` or `last_error`. |
| R004 | quality-attribute | validated | M028/S05 | M028/S02, M028/S06, M028/S07 | Validated by M028/S07 through live recovery proof on `reference-backend/`: `cargo run -p meshc -- build reference-backend`, `cargo run -p meshc -- fmt --check reference-backend`, `cargo run -p meshc -- test reference-backend`, and ignored compiler e2e proofs `e2e_reference_backend_worker_crash_recovers_job`, `e2e_reference_backend_worker_restart_is_visible_in_health`, `e2e_reference_backend_process_restart_recovers_inflight_job`, `e2e_reference_backend_migration_status_and_apply`, and `e2e_reference_backend_deploy_artifact_smoke`. The proof shows a real degraded/recovering window, restart-count and exit metadata on `/health`, stale-cutoff reclaim of abandoned `processing` rows, and exact-once completion after both worker crash and whole-process restart on the canonical backend path. |
| R005 | launchability | validated | M028/S04 | M028/S06 | Validated by M028/S04 through live native-deployment proof for `reference-backend/`: `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture`, staged bundle verification via `reference-backend/scripts/stage-deploy.sh`, `cargo test -p meshc e2e_self_contained_binary -- --nocapture`, ignored operational proof `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`, missing-artifact diagnostics in `apply-deploy-migrations.sh`, and operator-facing docs/env contract in `reference-backend/README.md` and `reference-backend/.env.example`. Proof covers build-host staging, runtime-host `psql` migration apply without `meshc`, staged binary startup outside the repo root, `/health`, job creation + processing, `_mesh_migrations` recording, and log redaction of `DATABASE_URL`. |
| R006 | quality-attribute | validated | M028/S03 | M030/S01 (provisional), M030/S02 (provisional) | S03 closure reran the full tooling trust gate on `reference-backend/`: `cargo test -p mesh-fmt -- --nocapture`, `cargo test -p meshc --test e2e_fmt -- --nocapture`, `cargo run -p meshc -- fmt --check reference-backend`, `cargo run -p meshc -- test reference-backend`, `! cargo run -p meshc -- test --coverage reference-backend`, `cargo test -p meshc --test tooling_e2e -- --nocapture`, `cargo test -p meshc --test e2e_lsp -- --nocapture`, `cargo test -p mesh-lsp -- --nocapture`, and the stale-string sweep over README/website/VS Code/reference-backend docs all passed. |
| R007 | launchability | active | M030/S01 (provisional) | M030/S02 (provisional) | mapped |
| R008 | launchability | active | M028/S06 | M028/S01, M028/S03, M028/S04, M028/S05 | mapped |
| R009 | differentiator | validated | M028/S06 | M028/S01, M028/S02, M028/S05, M028/S07 | Validated by M028/S07 on top of the existing S01-S06 proof surface. The `reference-backend/` package now has green end-to-end build/fmt/test, migration/deploy smoke, worker-crash recovery, restart-visibility, and whole-process restart proofs in `compiler/meshc/tests/e2e_reference_backend.rs`, so Mesh is being judged through one real backend that exercises compiler, runtime, HTTP, Postgres, migrations, background jobs, and recovery instead of isolated subsystem demos. |
| R010 | differentiator | active | M029/S01 (provisional) | M028/S04, M028/S06, M029/S02 (provisional) | mapped |
| R011 | differentiator | active | M029/S02 (provisional) | M029/S03 (provisional) | mapped |
| R012 | core-capability | active | M031/S01 (provisional) | M031/S02 (provisional), M031/S03 (provisional) | mapped |
| R013 | constraint | active | M023/S01 | M027/S01 (provisional) | mapped |
| R014 | constraint | active | M023/S02 | M023/S06 | mapped |
| R020 | operability | deferred | none | none | unmapped |
| R021 | admin/support | deferred | none | none | unmapped |
| R022 | operability | deferred | M027/S02 (provisional) | none | unmapped |
| R030 | anti-feature | out-of-scope | none | none | n/a |
| R031 | anti-feature | out-of-scope | none | none | n/a |
| R032 | constraint | out-of-scope | none | none | n/a |
| R033 | constraint | out-of-scope | none | none | n/a |

## Coverage Summary

- Active requirements: 8
- Mapped to slices: 8
- Validated: 6 (R001, R002, R003, R004, R005, R006)
- Unmapped active requirements: 0
