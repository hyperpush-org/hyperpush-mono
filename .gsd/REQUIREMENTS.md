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
- Notes: This sits after the backend trust baseline but is already part of the capability contract.

### R010 — The project can point to specific ways Mesh is easier to deploy, measurably fast, and nicer for backend development rather than vaguely claiming it is "better than Elixir."
- Class: differentiator
- Status: active
- Description: The project can point to specific ways Mesh is easier to deploy, measurably fast, and nicer for backend development rather than vaguely claiming it is "better than Elixir."
- Why it matters: The comparison target is clear, but the comparison needs grounded evidence rather than rhetoric.
- Source: user
- Primary owning slice: M032/S05
- Supporting slices: M028/S04, M028/S06, M033/S05 (provisional)
- Validation: mapped
- Notes: M032 and M033 sharpen this by replacing folklore and improving the data layer through real dogfood pressure.

### R011 — New language/runtime work after M028 should come from real backend friction discovered while using Mesh for actual backend code.
- Class: differentiator
- Status: active
- Description: New language/runtime work after M028 should come from real backend friction discovered while using Mesh for actual backend code.
- Why it matters: This keeps the project from chasing clever language features that do not improve the target use case.
- Source: user
- Primary owning slice: M032/S01
- Supporting slices: M032/S02, M032/S03, M033/S02 (provisional)
- Validation: mapped
- Notes: The M032 wave now advances this through the S02 inferred-export repair and the S03 direct request/handler/control-flow dogfood cleanup in mesher.

### R035 — Comments in `mesher/` that claim a Mesh limitation or workaround must reflect current verified reality, not stale folklore.
- Class: quality-attribute
- Status: active
- Description: Comments in `mesher/` that claim a Mesh limitation or workaround must reflect current verified reality, not stale folklore.
- Why it matters: Stale limitation comments make Mesh look weaker than it is and hide the real regression surface.
- Source: execution
- Primary owning slice: M032/S01
- Supporting slices: M032/S03, M032/S05
- Validation: mapped
- Notes: S01 classified the stale-vs-real workaround families, S03 retired the audited request/handler/control-flow folklore in mesher, and S05 still owns the final retained-limit ledger plus integrated closeout proof.

### R036 — The ORM and migration surfaces should keep a neutral baseline API while allowing explicit PG or SQLite extras when the underlying capability is not honestly portable.
- Class: core-capability
- Status: active
- Description: The ORM and migration surfaces should keep a neutral baseline API while allowing explicit PG or SQLite extras when the underlying capability is not honestly portable.
- Why it matters: Fake portability preserves raw SQL and hides capability boundaries instead of making them explicit.
- Source: user
- Primary owning slice: M033/S01 (provisional)
- Supporting slices: M033/S02, M033/S04 (provisional)
- Validation: mapped
- Notes: The immediate implementation pressure is PG-first because that is what `mesher/` actually uses now.

### R037 — Mesh should expose PG-specific query and migration surfaces for the cases `mesher/` actually needs today: JSONB-heavy data access, expression-heavy updates, full-text search, crypto helpers, and partition-related DDL.
- Class: integration
- Status: active
- Description: Mesh should expose PG-specific query and migration surfaces for the cases `mesher/` actually needs today: JSONB-heavy data access, expression-heavy updates, full-text search, crypto helpers, and partition-related DDL.
- Why it matters: Mesher's current escape hatches are concentrated in real PostgreSQL features, not generic SQL.
- Source: execution
- Primary owning slice: M033/S02 (provisional)
- Supporting slices: M033/S03, M033/S04 (provisional)
- Validation: mapped
- Notes: This is not a mandate to eliminate every escape hatch, only to cover the honest recurring pressure points.

### R038 — After M033, `mesher/` should use stronger Mesh ORM and migration surfaces for the cases they honestly cover, while retaining only a short justified keep-list of raw SQL and DDL escape hatches.
- Class: quality-attribute
- Status: active
- Description: After M033, `mesher/` should use stronger Mesh ORM and migration surfaces for the cases they honestly cover, while retaining only a short justified keep-list of raw SQL and DDL escape hatches.
- Why it matters: The goal is a better platform and cleaner dogfood, not a purity metric that damages the app or the API.
- Source: user
- Primary owning slice: M033/S03 (provisional)
- Supporting slices: M033/S04, M033/S05 (provisional)
- Validation: mapped
- Notes: Behavior and data shape should stay stable unless a narrow change is unavoidable.

### R039 — Mesh migrations should cover the recurring schema and partition-management cases that force `mesher/` into raw DDL today, with explicit extras where needed.
- Class: launchability
- Status: active
- Description: Mesh migrations should cover the recurring schema and partition-management cases that force `mesher/` into raw DDL today, with explicit extras where needed.
- Why it matters: DDL gaps push real apps into hand-written SQL even when the patterns are common and stable.
- Source: user
- Primary owning slice: M033/S04 (provisional)
- Supporting slices: M033/S02 (provisional)
- Validation: mapped
- Notes: Catalog inspection and truly dynamic DDL may still remain escape hatches if the dedicated surfaces would be dishonest or overly specific.

### R040 — The M033 data-layer design should be shaped so SQLite-specific extras can be added later without backing out a PG-only abstraction.
- Class: constraint
- Status: active
- Description: The M033 data-layer design should be shaped so SQLite-specific extras can be added later without backing out a PG-only abstraction.
- Why it matters: The user wants a neutral code path with explicit vendor extras, not a one-off Postgres trap.
- Source: user
- Primary owning slice: M033/S01 (provisional)
- Supporting slices: M033/S02 (provisional)
- Validation: mapped
- Notes: SQLite extras are deferred implementation work, but the extension points should be designed now.

## Validated

### R001 — Mesh has an explicit definition of what "production ready language needs to have" means for this repo, and that baseline can be checked through concrete proof rather than vague claims.
- Class: launchability
- Status: validated
- Description: Mesh has an explicit definition of what "production ready language needs to have" means for this repo, and that baseline can be checked through concrete proof rather than vague claims.
- Why it matters: Without a baseline contract, the work turns into an endless feature list and nobody can tell whether Mesh actually became more trustworthy.
- Source: inferred
- Primary owning slice: M028/S01
- Supporting slices: M028/S06
- Validation: validated
- Notes: Validated by the shipped `reference-backend/` package, canonical startup contract, and compiler e2e proof around API + DB + migrations + jobs.

### R002 — Mesh can power a real backend shape with an HTTP API, persistent database state, migrations, and background jobs in one coherent flow.
- Class: core-capability
- Status: validated
- Description: Mesh can power a real backend shape with an HTTP API, persistent database state, migrations, and background jobs in one coherent flow.
- Why it matters: This is the first serious proof target for trusting Mesh for a real production app backend in any capacity.
- Source: user
- Primary owning slice: M028/S01
- Supporting slices: M028/S02, M028/S04, M028/S05, M028/S06
- Validation: validated
- Notes: Validated through live end-to-end verification of `reference-backend/`.

### R003 — The runtime path behind the canonical backend flow is exercised by automated verification strongly enough that the path is not just "implemented," but trusted.
- Class: quality-attribute
- Status: validated
- Description: The runtime path behind the canonical backend flow is exercised by automated verification strongly enough that the path is not just "implemented," but trusted.
- Why it matters: A backend language loses credibility quickly if its basic runtime surfaces only work in isolated or manual scenarios.
- Source: inferred
- Primary owning slice: M028/S02
- Supporting slices: M028/S06
- Validation: validated
- Notes: Validated by live Postgres-backed compiler e2e coverage on the reference backend.

### R004 — Mesh concurrency and supervision are proven under crash, restart, and failure-reporting scenarios instead of only being advertised as features.
- Class: quality-attribute
- Status: validated
- Description: Mesh concurrency and supervision are proven under crash, restart, and failure-reporting scenarios instead of only being advertised as features.
- Why it matters: "Concurrency exists but isn't trustworthy" was an explicit failure state.
- Source: user
- Primary owning slice: M028/S05
- Supporting slices: M028/S02, M028/S06, M028/S07
- Validation: validated
- Notes: Validated by M028/S07 through the live recovery proof path, though the closeout rerun still recorded residual flake in one serial acceptance proof.

### R005 — Mesh's native-binary workflow is proven through a deployment path that feels closer to shipping a Go app than to assembling a fragile language stack.
- Class: launchability
- Status: validated
- Description: Mesh's native-binary workflow is proven through a deployment path that feels closer to shipping a Go app than to assembling a fragile language stack.
- Why it matters: Easier deployment is one of the first ways Mesh should beat Elixir for this repo's target use case.
- Source: user
- Primary owning slice: M028/S04
- Supporting slices: M028/S06
- Validation: validated
- Notes: Validated by the boring native deployment proof for `reference-backend/`.

### R006 — Diagnostics, formatter, LSP, tests, and the coverage story are credible enough that a backend engineer can use Mesh daily without fighting the toolchain.
- Class: quality-attribute
- Status: validated
- Description: Diagnostics, formatter, LSP, tests, and the coverage story are credible enough that a backend engineer can use Mesh daily without fighting the toolchain.
- Why it matters: Better DX is part of the explicit comparison target against Elixir.
- Source: user
- Primary owning slice: M028/S03
- Supporting slices: M030/S01 (provisional), M030/S02 (provisional)
- Validation: validated
- Notes: The toolchain is judged against real backend code, not toy fixtures.

### R008 — Mesh documentation and examples show a production-style backend path and do not rely mainly on toy examples to make the language look ready.
- Class: launchability
- Status: validated
- Description: Mesh documentation and examples show a production-style backend path and do not rely mainly on toy examples to make the language look ready.
- Why it matters: The docs must prove real use, not only advertise features.
- Source: user
- Primary owning slice: M028/S06
- Supporting slices: M028/S01, M028/S03, M028/S04, M028/S05, M028/S07, M028/S08
- Validation: validated
- Notes: Validated through the reconciled production-proof surface.

### R009 — Mesh proves itself through a real reference backend that exercises the language as a backend platform instead of proving subsystems only in isolation.
- Class: differentiator
- Status: validated
- Description: Mesh proves itself through a real reference backend that exercises the language as a backend platform instead of proving subsystems only in isolation.
- Why it matters: Dogfooding is how the repo turns backend ambition into engineering pressure.
- Source: inferred
- Primary owning slice: M028/S06
- Supporting slices: M028/S01, M028/S02, M028/S05, M028/S07
- Validation: validated
- Notes: The reference backend remains the narrow proof target; `mesher/` is the broader pressure test.

### R013 — A blocking Mesh language/runtime/tooling limitation is not worked around indefinitely; it is fixed in Mesh and then used in mesher.
- Class: constraint
- Status: validated
- Description: A blocking Mesh language/runtime/tooling limitation is not worked around indefinitely; it is fixed in Mesh and then used in mesher.
- Why it matters: `mesher/` is a dogfooding vehicle as well as an application.
- Source: user
- Primary owning slice: M032/S02
- Supporting slices: M032/S03, M032/S04
- Validation: Validated by `cargo test -p meshc --test e2e m032_inferred -- --nocapture`, `cargo test -p meshc --test e2e m032_inferred_cross_module_identity -- --nocapture`, `cargo test -p meshc --test e2e e2e_cross_module_polymorphic -- --nocapture`, `cargo test -p meshc --test e2e e2e_cross_module_service -- --nocapture`, `bash scripts/verify-m032-s01.sh`, `cargo run -q -p meshc -- fmt --check mesher`, and `cargo run -q -p meshc -- build mesher` after moving `flush_batch` into `mesher/storage/writer.mpl` and importing it from `mesher/services/writer.mpl`.
- Notes: M032/S02 fixed the unconstrained inferred-export lowering bug in Mesh, replayed the old `xmod_identity` repro as a success path, and dogfooded the repaired module-boundary export from mesher instead of keeping the helper local to the service.

### R015 — `else if` chains produce the correct branch value instead of returning garbage or crashing on certain types.
- Class: core-capability
- Status: validated
- Description: `else if` chains produce the correct branch value instead of returning garbage or crashing on certain types.
- Why it matters: Silent wrong-value bugs in basic control flow undermine all language trust.
- Source: execution
- Primary owning slice: M031/S01
- Supporting slices: none
- Validation: validated
- Notes: Fixed by storing the resolved type in `infer_if`; backed by dedicated e2e coverage.

### R016 — Control-flow conditions ending in function calls parse correctly without workaround bindings.
- Class: core-capability
- Status: validated
- Description: Control-flow conditions ending in function calls parse correctly without workaround bindings.
- Why it matters: The old behavior forced awkward temporary variables and boolean comparison noise.
- Source: execution
- Primary owning slice: M031/S01
- Supporting slices: none
- Validation: validated
- Notes: Fixed with parser context suppression for trailing closures in condition positions.

### R017 — Multiline function calls resolve to the correct type instead of collapsing to `()`.
- Class: core-capability
- Status: validated
- Description: Multiline function calls resolve to the correct type instead of collapsing to `()`.
- Why it matters: Formatting long calls should not change semantics.
- Source: execution
- Primary owning slice: M031/S01
- Supporting slices: none
- Validation: validated
- Notes: Fixed in the AST layer by filtering trivia tokens in multiline literals.

### R018 — Parenthesized multiline imports parse into the same AST shape as flat imports.
- Class: quality-attribute
- Status: validated
- Description: Parenthesized multiline imports parse into the same AST shape as flat imports.
- Why it matters: Long import lines were unreadable and a recurring dogfood pain point.
- Source: user
- Primary owning slice: M031/S02
- Supporting slices: none
- Validation: validated
- Notes: Parser and e2e coverage prove single-line, multiline, and trailing-comma import groups.

### R019 — `fn_call(a, b, c,)` and multiline trailing-comma call formatting work correctly.
- Class: quality-attribute
- Status: validated
- Description: `fn_call(a, b, c,)` and multiline trailing-comma call formatting work correctly.
- Why it matters: This is basic multiline ergonomics and diff hygiene.
- Source: inferred
- Primary owning slice: M031/S02
- Supporting slices: none
- Validation: validated
- Notes: Backed by parser, formatter, and dedicated e2e coverage.

### R023 — `reference-backend/` has zero `let _ =` side-effect bindings, no `== true` noise, struct update syntax, and idiomatic pipe usage.
- Class: quality-attribute
- Status: validated
- Description: `reference-backend/` has zero `let _ =` side-effect bindings, no `== true` noise, struct update syntax, and idiomatic pipe usage.
- Why it matters: The reference backend is the primary proof surface and should model good Mesh code.
- Source: user
- Primary owning slice: M031/S03
- Supporting slices: none
- Validation: validated
- Notes: Proven by grep gates plus build, formatter, project tests, and e2e verification.

### R024 — `mesher/` has zero `let _ =` side-effect bindings, interpolation where appropriate, multiline imports, and idiomatic pipe usage.
- Class: quality-attribute
- Status: validated
- Description: `mesher/` has zero `let _ =` side-effect bindings, interpolation where appropriate, multiline imports, and idiomatic pipe usage.
- Why it matters: `mesher/` is the broader dogfood app and should reflect real language usability.
- Source: user
- Primary owning slice: M029/S02
- Supporting slices: M029/S01, M029/S03
- Validation: validated
- Notes: Validated by grep gates plus `meshc fmt --check mesher` and `meshc build mesher`.

### R025 — The suite covers bare expression statements, fn-call control-flow conditions, multiline calls/imports, trailing commas, service-handler struct updates, and related dogfood patterns.
- Class: quality-attribute
- Status: validated
- Description: The suite covers bare expression statements, fn-call control-flow conditions, multiline calls/imports, trailing commas, service-handler struct updates, and related dogfood patterns.
- Why it matters: These patterns had little or no regression coverage before the M031 wave.
- Source: user
- Primary owning slice: M031/S05
- Supporting slices: M031/S01, M031/S02
- Validation: validated
- Notes: Full suite baseline is 328 tests with the known try-family failures explicitly tracked in project knowledge.

### R026 — Formatter output keeps `Api.Router` intact and does not collapse or corrupt multiline import groups.
- Class: quality-attribute
- Status: validated
- Description: Formatter output keeps `Api.Router` intact and does not collapse or corrupt multiline import groups.
- Why it matters: Formatter corruption destroys trust quickly and blocks dogfood cleanup.
- Source: execution
- Primary owning slice: M029/S01
- Supporting slices: none
- Validation: validated
- Notes: Backed by formatter library tests, exact-output CLI tests, and clean `fmt --check` runs on both dogfood codebases.

### R027 — `reference-backend/` source files keep canonical dotted module paths and stay formatter-clean.
- Class: quality-attribute
- Status: validated
- Description: `reference-backend/` source files keep canonical dotted module paths and stay formatter-clean.
- Why it matters: Formatter-induced import corruption in the primary backend proof surface undermines tooling trust.
- Source: execution
- Primary owning slice: M029/S01
- Supporting slices: none
- Validation: validated
- Notes: Proven by repaired source plus `fmt --check reference-backend` and dot-path grep gates.

## Deferred

### R012 — Mesh should continue from the reference-backend and mesher proof surfaces toward broader backend forms like long-running services, realtime systems, and distributed backends.
- Class: core-capability
- Status: deferred
- Description: Mesh should continue from the reference-backend and mesher proof surfaces toward broader backend forms like long-running services, realtime systems, and distributed backends.
- Why it matters: The long-term vision is broader than one app shape.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred behind the M032/M033 dogfood truth and data-layer work.

### R014 — The creator-token treasury and fund product loop remains part of the broader repo backlog but is not part of the current Mesh platform milestone sequence.
- Class: constraint
- Status: deferred
- Description: The creator-token treasury and fund product loop remains part of the broader repo backlog but is not part of the current Mesh platform milestone sequence.
- Why it matters: It keeps the current planning wave focused on Mesh and dogfood credibility instead of splitting attention across two unrelated fronts.
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: The older product draft milestones remain deferred while the repo focus stays on Mesh maturity.

### R020 — Mesh eventually offers a stronger debugger/profiler/trace surface suitable for deeper production diagnostics.
- Class: operability
- Status: deferred
- Description: Mesh eventually offers a stronger debugger/profiler/trace surface suitable for deeper production diagnostics.
- Why it matters: Mature backend ecosystems are judged heavily on observability and debugging, but this should not swallow the current dogfood wave.
- Source: research
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: Deferred until the current trust and data-layer work lands.

### R021 — Registry, publishing flow, package trust, and ecosystem polish should rise from credible to mature.
- Class: admin/support
- Status: deferred
- Description: Registry, publishing flow, package trust, and ecosystem polish should rise from credible to mature.
- Why it matters: It matters for adoption, but it should not displace the present dogfood and ORM pressure work.
- Source: research
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: M030 keeps the nearer-term package and tooling trust work active.

### R022 — Operators eventually get richer admin controls, manual retries, and deeper operational tooling.
- Class: operability
- Status: deferred
- Description: Operators eventually get richer admin controls, manual retries, and deeper operational tooling.
- Why it matters: It improves long-term operability once the core platform and data-path ergonomics are stronger.
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: Day-one requirement is failure visibility and trustworthy dogfood, not a full operator cockpit.

### R041 — SQLite-specific ORM and migration extras should be implemented after the neutral core and PG extras are proven on real pressure.
- Class: integration
- Status: deferred
- Description: SQLite-specific ORM and migration extras should be implemented after the neutral core and PG extras are proven on real pressure.
- Why it matters: The design should leave a clean SQLite path, but current implementation pressure is coming from Postgres-backed mesher work.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: M033 should shape the extension points so this later work is straightforward.

## Out of Scope

### R030 — The current planning wave is not a frontend-first language push.
- Class: anti-feature
- Status: out-of-scope
- Description: The current planning wave is not a frontend-first language push.
- Why it matters: This prevents scope confusion and preserves the explicit backend bias from the discussion.
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Mesh remains general-purpose, but the proof and planning direction are backend-led.

### R031 — M032 should not turn into a wide language-design sweep unrelated to proven mesher blockers.
- Class: anti-feature
- Status: out-of-scope
- Description: M032 should not turn into a wide language-design sweep unrelated to proven mesher blockers.
- Why it matters: This keeps the milestone honest and dogfood-driven.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: New syntax or broad semantics changes need a stronger justification than a stale comment.

### R032 — The repo will not claim production readiness based only on feature lists, benchmarks, or toy examples.
- Class: constraint
- Status: out-of-scope
- Description: The repo will not claim production readiness based only on feature lists, benchmarks, or toy examples.
- Why it matters: This blocks exactly the weak proof mode the project rejects.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Honest proof remains non-negotiable.

### R033 — Native mobile is not part of the current Mesh platform milestone sequence.
- Class: constraint
- Status: out-of-scope
- Description: Native mobile is not part of the current Mesh platform milestone sequence.
- Why it matters: It keeps attention on the backend and dogfood platform surfaces.
- Source: inferred
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Web and backend flows remain the primary proof surfaces.

### R034 — M033 should not chase broad generic data-layer abstractions that do not retire a real pressure point from `mesher/`.
- Class: anti-feature
- Status: out-of-scope
- Description: M033 should not chase broad generic data-layer abstractions that do not retire a real pressure point from `mesher/`.
- Why it matters: Over-generalizing the ORM would make the API worse while still missing the real dogfood gaps.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: The right bar is honest pressure coverage, not a giant clever DSL.

### R043 — The success bar is pragmatic reduction with a justified keep-list, not raw-SQL purity.
- Class: anti-feature
- Status: out-of-scope
- Description: The success bar is pragmatic reduction with a justified keep-list, not raw-SQL purity.
- Why it matters: A fake zero target would incentivize dishonest abstractions and brittle rewrites.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Remaining escape hatches should be short, named, and justified.

### R044 — `mesher/` should remain behaviorally stable from the product point of view while the platform underneath it improves.
- Class: constraint
- Status: out-of-scope
- Description: `mesher/` should remain behaviorally stable from the product point of view while the platform underneath it improves.
- Why it matters: This keeps the milestones focused on Mesh and data-layer capability rather than smuggling in a product redesign.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Narrow app changes are acceptable only when required to dogfood the repaired or expanded platform path.

## Traceability

| ID | Class | Status | Primary owner | Supporting | Proof |
|---|---|---|---|---|---|
| R001 | launchability | validated | M028/S01 | M028/S06 | validated |
| R002 | core-capability | validated | M028/S01 | M028/S02, M028/S04, M028/S05, M028/S06 | validated |
| R003 | quality-attribute | validated | M028/S02 | M028/S06 | validated |
| R004 | quality-attribute | validated | M028/S05 | M028/S02, M028/S06, M028/S07 | validated |
| R005 | launchability | validated | M028/S04 | M028/S06 | validated |
| R006 | quality-attribute | validated | M028/S03 | M030/S01 (provisional), M030/S02 (provisional) | validated |
| R007 | launchability | active | M030/S01 (provisional) | M030/S02 (provisional) | mapped |
| R008 | launchability | validated | M028/S06 | M028/S01, M028/S03, M028/S04, M028/S05, M028/S07, M028/S08 | validated |
| R009 | differentiator | validated | M028/S06 | M028/S01, M028/S02, M028/S05, M028/S07 | validated |
| R010 | differentiator | active | M032/S05 | M028/S04, M028/S06, M033/S05 (provisional) | mapped |
| R011 | differentiator | active | M032/S01 | M032/S02, M032/S03, M033/S02 (provisional) | mapped |
| R012 | core-capability | deferred | none | none | unmapped |
| R013 | constraint | validated | M032/S02 | M032/S03, M032/S04 | Validated by `cargo test -p meshc --test e2e m032_inferred -- --nocapture`, `cargo test -p meshc --test e2e m032_inferred_cross_module_identity -- --nocapture`, `cargo test -p meshc --test e2e e2e_cross_module_polymorphic -- --nocapture`, `cargo test -p meshc --test e2e e2e_cross_module_service -- --nocapture`, `bash scripts/verify-m032-s01.sh`, `cargo run -q -p meshc -- fmt --check mesher`, and `cargo run -q -p meshc -- build mesher` after moving `flush_batch` into `mesher/storage/writer.mpl` and importing it from `mesher/services/writer.mpl`. |
| R014 | constraint | deferred | none | none | unmapped |
| R015 | core-capability | validated | M031/S01 | none | validated |
| R016 | core-capability | validated | M031/S01 | none | validated |
| R017 | core-capability | validated | M031/S01 | none | validated |
| R018 | quality-attribute | validated | M031/S02 | none | validated |
| R019 | quality-attribute | validated | M031/S02 | none | validated |
| R020 | operability | deferred | none | none | unmapped |
| R021 | admin/support | deferred | none | none | unmapped |
| R022 | operability | deferred | none | none | unmapped |
| R023 | quality-attribute | validated | M031/S03 | none | validated |
| R024 | quality-attribute | validated | M029/S02 | M029/S01, M029/S03 | validated |
| R025 | quality-attribute | validated | M031/S05 | M031/S01, M031/S02 | validated |
| R026 | quality-attribute | validated | M029/S01 | none | validated |
| R027 | quality-attribute | validated | M029/S01 | none | validated |
| R030 | anti-feature | out-of-scope | none | none | n/a |
| R031 | anti-feature | out-of-scope | none | none | n/a |
| R032 | constraint | out-of-scope | none | none | n/a |
| R033 | constraint | out-of-scope | none | none | n/a |
| R034 | anti-feature | out-of-scope | none | none | n/a |
| R035 | quality-attribute | active | M032/S01 | M032/S03, M032/S05 | mapped |
| R036 | core-capability | active | M033/S01 (provisional) | M033/S02, M033/S04 (provisional) | mapped |
| R037 | integration | active | M033/S02 (provisional) | M033/S03, M033/S04 (provisional) | mapped |
| R038 | quality-attribute | active | M033/S03 (provisional) | M033/S04, M033/S05 (provisional) | mapped |
| R039 | launchability | active | M033/S04 (provisional) | M033/S02 (provisional) | mapped |
| R040 | constraint | active | M033/S01 (provisional) | M033/S02 (provisional) | mapped |
| R041 | integration | deferred | none | none | unmapped |
| R043 | anti-feature | out-of-scope | none | none | n/a |
| R044 | constraint | out-of-scope | none | none | n/a |

## Coverage Summary

- Active requirements: 9
- Mapped to slices: 9
- Validated: 19 (R001, R002, R003, R004, R005, R006, R008, R009, R013, R015, R016, R017, R018, R019, R023, R024, R025, R026, R027)
- Unmapped active requirements: 0
