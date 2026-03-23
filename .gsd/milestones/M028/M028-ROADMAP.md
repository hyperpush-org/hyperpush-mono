# M028: Language Baseline Audit & Hardening

**Vision:** Turn Mesh’s broad existing backend surface into something a developer can trust for a real production app backend in any capacity, starting with an honest API + DB + migrations + background jobs proof path that pushes easier deployment, raw performance, and better DX instead of vague claims.

## Success Criteria

- A reference Mesh backend with API + DB + migrations + background jobs can be built, run, and verified end-to-end from this repo.
- The reference backend’s failure and recovery behavior is exercised strongly enough that concurrency does not merely exist — it is trustworthy.
- The reference backend can be built into a native binary and deployed through a boring documented workflow closer to a Go app than to a fragile language stack.
- Docs/examples point to the real backend proof path and stop relying mainly on toy examples to imply readiness.

## Key Risks / Unknowns

- Mesh’s claimed backend surface may not survive one real end-to-end backend proof path — the repo could still contain broad capability with shallow assembly proof.
- Concurrency may “exist but isn’t trustworthy” under crash/restart conditions — that would immediately break the Elixir comparison target.
- Tooling may lag the backend/runtime enough that daily backend work still feels fragile — that would block the DX goal even if runtime behavior improves.
- Docs/examples may still not prove real use — that would leave external evaluators unconvinced even after technical fixes land.

## Proof Strategy

- Claimed backend surface may not hold end-to-end → retire in S01 by proving one concrete reference backend shape instead of planning against abstractions.
- Runtime correctness on the golden path may be weaker than repo claims suggest → retire in S02 by exercising HTTP, DB, migrations, and jobs with real automated proof and fixing the most damaging blockers.
- Daily-driver DX may still feel fragile → retire in S03 by raising fmt/diagnostics/test/LSP/coverage credibility against the same reference backend workflow.
- Native deployment may still require too much ceremony → retire in S04 by proving a boring binary deployment path with smoke verification.
- Concurrency trust may collapse under failures → retire in S05 by proving supervision, restart, and failure visibility behavior against the reference backend.
- Docs/examples may still be toy-heavy → retire in S06 by shipping an honest production-style proof surface tied to the real backend path.

## Verification Classes

- Contract verification: Rust tests, Mesh E2E tests, artifact checks, docs build checks, and reference-app command verification
- Integration verification: real reference backend exercising compiler → runtime → HTTP → DB → migrations → background jobs together
- Operational verification: binary build/startup, migration apply, service/job restart behavior, failure visibility, and deployment smoke verification
- UAT / human verification: final judgment on whether the docs/examples and proof surface feel honest enough for external backend evaluators

## Milestone Definition of Done

This milestone is complete only when all are true:

- all slice deliverables are complete
- the reference backend path is actually wired together across compiler, runtime, HTTP, DB, migrations, and background jobs
- the real backend entrypoint exists and is exercised as a native-binary workflow
- success criteria are re-checked against live behavior, not just artifacts or isolated fixtures
- final integrated acceptance scenarios pass

## Requirement Coverage

- Covers: R001, R002, R003, R004, R005, R006, R008, R009
- Partially covers: R010
- Leaves for later: R007, R011, R012, R020, R021
- Orphan risks: none

## Slices

- [x] **S01: Canonical Backend Golden Path** `risk:high` `depends:[]`
  > After this: Mesh has one concrete reference backend with API + DB + migrations + background jobs, so the milestone stops arguing in abstractions and has a real proof target.

- [x] **S02: Runtime Correctness on the Golden Path** `risk:high` `depends:[S01]`
  > After this: the reference backend’s HTTP/DB/migration/job path is exercised by automated proof, and the most credibility-damaging blockers on that path are fixed.

- [x] **S03: Daily-Driver Tooling Trust** `risk:medium` `depends:[S01]`
  > After this: the same backend can be developed with materially stronger fmt/diagnostics/test/LSP/coverage surfaces, so DX claims rest on a real workflow instead of toy files.

- [x] **S04: Boring Native Deployment** `risk:medium` `depends:[S01,S02]`
  > After this: the reference backend can be built into a native binary and deployed with a boring documented path plus smoke verification closer to Go expectations.

- [ ] **S05: Supervision, Recovery, and Failure Visibility** `risk:high` `depends:[S01,S02]`
  > After this: supervised jobs/services in the reference backend survive crashes predictably, and failures are visible instead of hiding in logs-only behavior.

- [ ] **S06: Honest Production Proof and Documentation** `risk:medium` `depends:[S01,S02,S03,S04,S05]`
  > After this: Mesh has a production-style backend proof surface — reference app, docs, examples, and verification — rather than toy-only evidence.

## Boundary Map

### S01 → S02

Produces:
- reference backend app skeleton with one real HTTP entrypoint, one database-backed persistence path, one migration set, and one background-job path
- canonical commands for build, run, migrate, and test against that backend
- backend contract for startup inputs, health/proof expectations, and persisted record shape

Consumes:
- nothing (first slice)

### S01 → S03

Produces:
- a real backend project shape large enough to exercise diagnostics, formatting, tests, LSP indexing, and coverage expectations honestly
- canonical backend workflow commands that tooling improvements can target and verify

Consumes:
- nothing (first slice)

### S02 → S04

Produces:
- verified runtime startup contract for the reference backend, including binary output, required environment/config inputs, and smoke-check expectations
- build/run proof that the backend path actually assembles under native compilation

Consumes from S01:
- reference backend app skeleton
- migration set and background-job path

### S02 → S05

Produces:
- known-good failure scenarios for the reference backend’s jobs/services
- verified behavior for crash isolation, restart, and surfaced failure state on the golden path

Consumes from S01:
- reference backend app skeleton
- canonical backend commands

### S03 → S06

Produces:
- tooling proof surfaces tied to the reference backend: diagnostics expectations, formatter behavior, test workflow, LSP behavior, and coverage story
- command-level documentation for the real backend workflow rather than toy-only examples

Consumes from S01:
- real backend project shape

### S04 → S06

Produces:
- documented native deployment path and smoke verification checklist for the reference backend
- artifact expectations for the built binary and runtime startup flow

Consumes from S01:
- backend build target

Consumes from S02:
- verified runtime startup contract

### S05 → S06

Produces:
- documented supervision/recovery proof and failure-visibility outputs for the reference backend
- final concurrency trust evidence tied to the same backend path used in the docs/examples

Consumes from S01:
- background-job and service paths

Consumes from S02:
- verified golden-path runtime behavior
