# S01: Modernize Mesher bootstrap and maintainer run path

**Goal:** Refit Mesher onto current Mesh bootstrap/config/runtime patterns and give it the maintainer-facing run/runbook surface that replaces `reference-backend/README.md` for deeper internal work.
**Demo:** After this: A maintainer can follow the Mesher runbook to migrate, build, and run Mesher against Postgres on the current runtime contract, and the live proof rails show readiness through Mesher’s real app surface.

## Tasks
- [x] **T01: Moved Mesher startup to the scaffold-style env contract and replaced app-owned cluster bootstrap with Node.start_from_env().** — Replace Mesher’s app-owned bootstrap with the scaffold-style env contract so the deeper reference app stops teaching stale runtime patterns.

- Why: `mesher/main.mpl` still hardcodes the Postgres DSN and bootstraps distribution through `MESHER_NODE_NAME` / `MESHER_COOKIE` / `MESHER_PEERS`, which makes Mesher a stale internal app instead of the maintained deeper reference surface promised by R119.
- Files: `mesher/main.mpl`, `mesher/config.mpl`, `mesher/tests/config.test.mpl`, `mesher/.env.example`
- Do: add config helpers for the Mesher env contract, rewrite startup around `DATABASE_URL` + validated ports/rate-limit values, open the pool before bootstrapping the runtime, replace app-owned node startup with `Node.start_from_env()`, and keep standalone + partition-bootstrap behavior truthful without leaking secrets.
- Verify: `cargo run -q -p meshc -- test mesher/tests` and `cargo run -q -p meshc -- build mesher`
- Done when: Mesher no longer contains a hardcoded DSN or app-owned peer bootstrap path, missing/invalid config fails closed before listener startup, and package tests pin the env contract.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| Env inputs for `DATABASE_URL`, `PORT`, `MESHER_WS_PORT`, and rate-limit settings | Log an explicit config error and stop before pool open or HTTP/WS startup. | N/A — local env lookup only. | Reject empty or non-positive ints instead of silently defaulting to nonsense values. |
| `Pool.open(...)` startup | Surface the DB failure without echoing the URL and do not start the runtime or listeners. | Exit without claiming readiness. | Treat unexpected pool errors as startup failure, not as fallback-to-standalone behavior. |
| `Node.start_from_env()` runtime bootstrap | Surface bootstrap failure separately from app config errors. | Do not hang before HTTP start; fail before claiming runtime ready. | Reject malformed cluster env through runtime-owned error text instead of keeping app-owned peer wiring. |

## Load Profile

- **Shared resources**: Postgres pool, HTTP/WS listener ports, runtime bootstrap state.
- **Per-operation cost**: one pool open, one runtime bootstrap, one partition bootstrap pass, and listener startup.
- **10x breakpoint**: startup confusion from bad env or silent defaults arrives before traffic load does, so fail-closed validation and logging are the protection.

## Negative Tests

- **Malformed inputs**: missing `DATABASE_URL`, invalid `PORT`, invalid `MESHER_WS_PORT`, and zero/negative rate-limit env.
- **Error paths**: Postgres connect failure, runtime bootstrap failure, and no-cluster boots with `MESH_*` env absent.
- **Boundary conditions**: truthful default values for local dev and a clean standalone boot when clustered env is intentionally omitted.

## Steps

1. Add `mesher/config.mpl` plus a package test file that pins env-key names and error-message helpers.
2. Rewrite `mesher/main.mpl` around config validation -> pool open -> `Node.start_from_env()` -> service/listener startup.
3. Remove the `MESHER_NODE_NAME` / `MESHER_COOKIE` / `MESHER_PEERS` path and replace it with runtime-owned bootstrap/failure logging.
4. Add `mesher/.env.example` that matches the new maintainer run contract without embedding secrets.

## Must-Haves

- [ ] `DATABASE_URL` is required and never hardcoded in Mesher startup.
- [ ] HTTP and WS ports are env-driven with explicit validation instead of silent garbage defaults.
- [ ] `Node.start_from_env()` is the only clustered bootstrap path in `mesher/main.mpl`.
- [ ] Startup logs expose config/runtime state without leaking DSNs or other secrets.
  - Estimate: 1h30m
  - Files: mesher/main.mpl, mesher/config.mpl, mesher/tests/config.test.mpl, mesher/.env.example
  - Verify: `cargo run -q -p meshc -- test mesher/tests` and `cargo run -q -p meshc -- build mesher`
- [x] **T02: Added a dedicated Mesher Postgres runtime proof rail with redacted `.tmp/m051-s01` artifacts.** — Give Mesher its own maintainer-oriented runtime acceptance target instead of burying confidence inside the older M033 storage slices.

- Why: the current Mesher proof is spread across M033 tests with duplicated helpers, a hardcoded DSN, and no dedicated artifact contract for the S01 bootstrap/run path.
- Files: `compiler/meshc/tests/e2e_m051_s01.rs`, `compiler/meshc/tests/support/m051_mesher.rs`, `compiler/meshc/tests/support/mod.rs`, `mesher/main.mpl`, `mesher/migrations/20260226000000_seed_default_org.mpl`
- Do: add a Mesher support harness that starts isolated Postgres, runs `meshc migrate mesher up`, builds and spawns Mesher with redacted artifacts, waits for readiness, and adds S01 tests for fail-closed missing `DATABASE_URL` plus a real app round-trip (`POST /api/v1/events` with the seeded key, then readback through Mesher’s real state/API surface); only touch runtime files if the new rail exposes missing observability or seed-contract gaps.
- Verify: `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`
- Done when: the new target runs real Mesher maintainer scenarios, retains redacted artifacts under `.tmp/m051-s01/`, and proves both config failure and Postgres-backed app readiness.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| Docker/Postgres container for Mesher e2e | Stop with retained container/setup logs and do not claim Mesher boot proof. | Preserve timeout artifacts and container logs before cleanup. | Reject partial startup/readiness states instead of treating any open port as success. |
| `meshc migrate mesher up` and `meshc build mesher` | Fail closed with command stdout/stderr and do not start the runtime. | Stop before HTTP proof and retain build/migrate logs. | Reject missing binary or missing migrated seed data as contract drift. |
| Mesher readiness + event ingest/readback surfaces | Archive raw response snapshots, stdout/stderr, and DB assertions so bootstrap drift, auth drift, and app-surface drift stay separable. | Keep the last observed HTTP/DB state and stop instead of retrying forever. | Reject non-JSON or wrong-status responses even if the process stayed up. |

## Load Profile

- **Shared resources**: Docker daemon, Postgres container, temp artifact dirs under `.tmp/m051-s01/`, and Mesher HTTP/WS ports.
- **Per-operation cost**: one DB container, one migrate run, one build, one runtime spawn, and a handful of HTTP/DB assertions.
- **10x breakpoint**: container startup/build time and artifact volume dominate first, so the harness should stay serial and reuse one runtime per scenario.

## Negative Tests

- **Malformed inputs**: missing `DATABASE_URL`, missing/invalid auth header, and malformed event JSON where the live proof needs it.
- **Error paths**: Postgres unavailable, migration failure, build failure, readiness timeout, wrong HTTP status/body, and secret leakage into artifacts.
- **Boundary conditions**: the seeded `default` project still resolves by slug, the first event creates issue state, and the proof does not collapse to a settings-only 200 response.

## Steps

1. Create `compiler/meshc/tests/support/m051_mesher.rs` with the minimal Mesher Postgres/build/run helpers needed for S01.
2. Add `compiler/meshc/tests/e2e_m051_s01.rs` with at least one fail-closed config test and one live Postgres-backed app round-trip test.
3. Retain redacted stdout/stderr/HTTP/DB artifacts under `.tmp/m051-s01/` so future agents can localize failures quickly.
4. Update `compiler/meshc/tests/support/mod.rs` and only touch Mesher runtime files if the new rail uncovers missing observability or seed-contract gaps.

## Must-Haves

- [ ] `cargo test -p meshc --test e2e_m051_s01 -- --nocapture` runs real Mesher maintainer scenarios, not 0 filtered tests.
- [ ] One scenario proves missing `DATABASE_URL` fails closed before readiness.
- [ ] One scenario proves migrate + build + run + event ingest + readback against Postgres through Mesher’s real app surface.
- [ ] Retained artifacts redact `DATABASE_URL` and other maintainer-supplied secrets.
  - Estimate: 2h
  - Files: compiler/meshc/tests/e2e_m051_s01.rs, compiler/meshc/tests/support/m051_mesher.rs, compiler/meshc/tests/support/mod.rs, mesher/main.mpl, mesher/migrations/20260226000000_seed_default_org.mpl
  - Verify: `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`
- [x] **T03: Published a package-local Mesher maintainer runbook and fail-closed verifier tied to the real Mesher runtime rail.** — Make the new Mesher contract usable by maintainers and keep it from drifting back toward tribal knowledge or `reference-backend` dependency.

- Why: S01 is not done when only tests know the right commands; maintainers need one repo-owned Mesher runbook and one verifier that keeps those commands, env keys, and live proof surfaces aligned while public docs stay untouched until S04.
- Files: `mesher/README.md`, `scripts/verify-m051-s01.sh`, `compiler/meshc/tests/e2e_m051_s01.rs`, `mesher/.env.example`, `mesher/main.mpl`, `mesher/migrations/20260226000000_seed_default_org.mpl`
- Do: write a maintainer-facing Mesher README with repo-root migrate/build/run commands, env contract, and a live seed-event smoke; add `scripts/verify-m051-s01.sh` that replays Mesher package tests, build, the dedicated S01 e2e target, and fail-closed README contract checks; keep the task scoped to maintainer surfaces and do not retarget public README/VitePress docs here.
- Verify: `bash scripts/verify-m051-s01.sh`
- Done when: `mesher/README.md` is the canonical Mesher maintainer runbook, the script fails closed on README/runtime drift, and a maintainer can replay the documented commands without consulting `reference-backend/README.md`.

## Failure Modes

| Dependency | On error | On timeout | On malformed response |
|------------|----------|-----------|----------------------|
| README command/env contract vs code/test surfaces | Fail on the first missing or stale command/env key instead of letting docs drift silently. | N/A — local text and command checks only. | Reject stale route/header names or retired `reference-backend` dependencies as contract drift. |
| `cargo run -q -p meshc -- test mesher/tests`, `cargo run -q -p meshc -- build mesher`, and `cargo test -p meshc --test e2e_m051_s01 -- --nocapture` | Stop at the failing phase and point to the phase log rather than papering over runtime drift. | Preserve the failing phase log and phase marker for diagnosis. | Reject 0-test executions or missing artifact pointers as verifier drift. |
| Maintainer live smoke contract (`POST /api/v1/events` + seeded default project/readback) | Keep the README honest by checking the documented route/header names against the proven e2e contract. | Preserve the e2e artifact pointer instead of pretending the README alone proved runtime truth. | Reject mismatched header names, stale seed-key text, or non-existent readback endpoints. |

## Load Profile

- **Shared resources**: README contract text, the new e2e target, Mesher build output, and `.tmp/m051-s01/verify` logs.
- **Per-operation cost**: one package-test run, one build, one dedicated e2e target, and a set of text/contract assertions.
- **10x breakpoint**: repeated full e2e replays dominate, so the script should keep static checks cheap and run the live target only once.

## Negative Tests

- **Malformed inputs**: stale README command strings, wrong env-key names, wrong header names, or missing verifier files.
- **Error paths**: build/test target missing, e2e target runs 0 tests, README drifts back toward `reference-backend`, or the verifier stops checking the new maintainer pointer.
- **Boundary conditions**: Mesher stays maintainer-facing only in this slice; the script must not require public docs changes before S04.

## Steps

1. Write `mesher/README.md` as the deeper maintainer runbook, borrowing the useful runbook shape from `reference-backend/README.md` but swapping in Mesher’s new env/bootstrap/app-surface truth.
2. Document repo-root migrate/build/run commands plus a live event smoke that uses the seeded default org/project and dev API key.
3. Implement `scripts/verify-m051-s01.sh` to replay package tests, build, the dedicated S01 e2e target, and fail-closed README contract checks.
4. Keep the script scoped to maintainer surfaces; do not retarget public VitePress/README links in this task.

## Must-Haves

- [ ] `mesher/README.md` is the canonical maintainer Mesher runbook for migrate/build/run/live smoke.
- [ ] `scripts/verify-m051-s01.sh` replays the real S01 proof, not just static text checks.
- [ ] README commands/env keys/header names stay mechanically checked against code and tests.
- [ ] The task does not reintroduce Mesher as a public first-contact docs path before S04.
  - Estimate: 1h15m
  - Files: mesher/README.md, scripts/verify-m051-s01.sh, compiler/meshc/tests/e2e_m051_s01.rs, mesher/.env.example, mesher/main.mpl, mesher/migrations/20260226000000_seed_default_org.mpl
  - Verify: `bash scripts/verify-m051-s01.sh`
