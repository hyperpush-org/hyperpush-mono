# S05 Research: Integrated mesher proof and retained-limit ledger

## Summary

S05 is mostly a closeout slice, but it is **not** artifact-only.

Current repo state is already close to green closeout:

- `bash scripts/verify-m032-s01.sh` passed end-to-end during this scout pass.
- `cargo test -q -p meshc --test e2e m032_inferred -- --nocapture` passed (`2 passed`).
- `cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture` passed (`1 passed`).
- The stale-folklore phrases retired by S03/S04 are already gone from `mesher/`.

But S05 still has two real truth-surface jobs before it can publish the final ledger:

1. **Integrated proof + artifact closeout** for R010 / milestone completion.
2. **Two newly surfaced overbroad comments** that were not covered by S03/S04 and should be corrected before the final retained-limit ledger is written:
   - `mesher/ingestion/routes.mpl:299-302` overstates the bulk JSON limitation.
   - `mesher/services/writer.mpl:61-62` still blames “service dispatch codegen” for a helper extraction that current Mesh handles fine.

The wider ORM / migration pressure is also real, but it is **too broad for S05 to solve**. There are:

- **16** explicit `# ORM boundary:` comments across `mesher/storage/queries.mpl` and `mesher/storage/writer.mpl`
- **27** `Repo.query_raw(...)` / `Repo.execute_raw(...)` call sites in `mesher/`

That belongs in a **family-level handoff to M033**, not a one-line-at-a-time S05 cleanup pass.

This research followed the `debug-like-expert` skill’s core rule: **VERIFY, DON’T ASSUME.** I re-ran the live proof matrix first, then checked each surviving limitation comment against named tests, source seams, or a minimal repro before classifying it.

## Recommendation

Plan S05 in this order:

1. **Truth-surface cleanup first**
   - Narrow the bulk-array comment in `mesher/ingestion/routes.mpl` to the real current limitation.
   - Remove or rewrite the stale “service dispatch codegen” rationale in `mesher/services/writer.mpl`.
   - Do this **before** writing the retained-limit ledger, otherwise the final artifact will preserve false wording.

2. **Run the integrated closeout proof**
   - Reuse `scripts/verify-m032-s01.sh` as the authoritative milestone replay.
   - Re-run the named inferred-export and route-closure controls so the closeout artifact can point to stable proof names, not just one shell script.

3. **Write the S05 retained-limit ledger / closeout artifacts**
   - Separate the final output into:
     - **Solved / supported now** landmarks
     - **Still-real Mesh-language/tooling keep-sites**
     - **Still-real ORM / migration follow-on families for M033**
   - Keep the M032 ledger short by grouping the ORM/DDL pressure into families instead of itemizing all 16 `ORM boundary:` comments.

Natural task split:

- **Task A:** comment-truth repair in `mesher/ingestion/routes.mpl` and `mesher/services/writer.mpl` (optionally with one or two tiny durable proofs if the planner wants them)
- **Task B:** integrated proof replay + final S05 summary/UAT/roadmap/requirements closeout

## Requirements Targeted

- **R010** — **primary owner for this slice**. S05 is the point where the repo turns the repaired Mesher proof surface into a current, defensible evidence bundle instead of a loose set of earlier slice summaries.
- **Supports R035** — every remaining limitation/workaround comment in `mesher/` should be tied to current evidence, not folklore.
- **Protects validated R013** — the repaired `xmod_identity` path should stay in the final closeout evidence so the milestone does not accidentally regress back to the old failure story.
- **Supports R011** — the closeout remains anchored to real Mesher paths, not synthetic compiler-only claims.

## Skills Discovered

- **Loaded:** `debug-like-expert`
  - Applied rules:
    - **VERIFY, DON’T ASSUME** — every retained/stale classification below is tied to an actual proof surface.
    - **NO DRIVE-BY FIXES** — I did not assume the remaining comment surfaces were correct just because earlier slices were green.
    - **ANALYSIS-ONLY FIRST** — live proof was rerun before recommending any edits.
- **Already available but not needed:** `rust-best-practices`
- **Skill search performed:**
  - `npx skills find "PostgreSQL"`
- **Result:** no new skill installed. The returned skills were schema-design / optimization oriented; S05 is closeout proof + truth-surface curation, not a database redesign slice.

## Implementation Landscape

### A. The integrated proof surface is already green

**Authoritative script**

- `scripts/verify-m032-s01.sh`
  - Replays the full M032 matrix from repo root.
  - Covers:
    - supported paths (`request_query`, `xmod_from_json`, `service_call_case`, `cast_if_else`, `xmod_identity`)
    - retained failure / behavior paths (`nested_and`, `timer_service_cast`, live bare-route control, live closure-route failure)
    - `cargo run -q -p meshc -- fmt --check mesher`
    - `cargo run -q -p meshc -- build mesher`
  - Leaves artifacts under `.tmp/m032-s01/verify/` on drift.

**Named tests worth citing in final artifacts**

- `compiler/meshc/tests/e2e.rs:6830-6868`
  - `e2e_m032_supported_request_query`
  - `e2e_m032_supported_cross_module_from_json`
  - `e2e_m032_supported_service_call_case`
  - `e2e_m032_supported_cast_if_else`
- `compiler/meshc/tests/e2e.rs:6877-6909`
  - `m032_inferred_local_identity`
  - `m032_inferred_cross_module_identity`
- `compiler/meshc/tests/e2e.rs:6914-6935`
  - `e2e_m032_limit_nested_and`
  - `e2e_m032_limit_timer_service_cast`
- `compiler/meshc/tests/e2e_stdlib.rs:1992-2042`
  - `e2e_m032_route_bare_handler_control`
  - `e2e_m032_route_closure_runtime_failure`

**Observed during this scout pass**

```bash
bash scripts/verify-m032-s01.sh
cargo test -q -p meshc --test e2e m032_inferred -- --nocapture
cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture
```

Observed result:

- replay script passed end-to-end (`verify-m032-s01: ok`)
- inferred tests: `2 passed`
- route-closure runtime failure control: `1 passed`

**Planning consequence**

S05 does **not** need a large bug-fix task to get back to green. It starts from a passing matrix and should focus on truth-surface completion + artifact closeout.

### B. Still-real Mesh-language/tooling keep-sites

These are the current language/tooling limits that still deserve inclusion in the final retained-limit ledger.

#### B1. HTTP route closures are still a live runtime limitation

**Mesher site**

- `mesher/ingestion/routes.mpl:2`
  - `# Handlers are bare functions (HTTP routing does not support closures).`

**Proof surface**

- `compiler/meshc/tests/e2e_stdlib.rs:1992-2016` — `e2e_m032_route_bare_handler_control`
- `compiler/meshc/tests/e2e_stdlib.rs:2018-2042` — `e2e_m032_route_closure_runtime_failure`
- `scripts/verify-m032-s01.sh` live requests against `route_bare_server` and `route_closure_server`

**Why it matters**

This remains the best example of a compile-pass / runtime-fail limitation. S05 should keep warning future work not to classify HTTP closure support from build-only evidence.

#### B2. Nested `&&` in nested `if` blocks is still a codegen blocker

**Mesher site**

- `mesher/services/stream_manager.mpl:63`
  - `# AND helper for filter matching -- avoids && codegen issue inside nested if blocks.`

**Proof surface**

- `compiler/meshc/tests/e2e.rs:6914-6925` — `e2e_m032_limit_nested_and`
- `scripts/verify-m032-s01.sh` (`nested_and` step)

**Why it matters**

This is a real retained Mesh blocker and should stay in the final ledger until the compiler/codegen is fixed.

#### B3. `Timer.send_after(...)` still does not satisfy service cast dispatch

**Mesher sites**

- `mesher/services/writer.mpl:153-154`
- `mesher/ingestion/pipeline.mpl:81-82`

**Proof surface**

- `compiler/meshc/tests/e2e.rs:6927-6935` — `e2e_m032_limit_timer_service_cast`
- `scripts/verify-m032-s01.sh` (`timer_service_cast` step)

**Why it matters**

The current recursive `Timer.sleep(...)` ticker pattern is still a truthful keep-site and should stay in the ledger.

#### B4. Multi-statement `case` arms still require `-> do ... end`

**Mesher sites**

- `mesher/services/event_processor.mpl:105`
- `mesher/ingestion/fingerprint.mpl:53`
- `mesher/services/retention.mpl:8`
- `mesher/ingestion/pipeline.mpl:293`
- `mesher/api/team.mpl:59,78`

**Language seam**

- `compiler/mesh-parser/src/parser/expressions.rs:963-991`
  - `parse_match_arm(...)`
  - comment: `Parse arm body: a single expression, or a do...end block.`

**Important nuance**

This is the **weakest mechanized proof surface** in the current M032 bundle. There is parser-source evidence and multiple still-live helper comments, but there is no named M032 e2e test dedicated to case-arm body shape.

**Planning consequence**

S05 can still ledger this honestly, but if the planner wants fully durable proof instead of source-level evidence, it should add one small parser/CLI repro before rewriting any case-arm comments.

### C. Two overbroad comments are still left and should be corrected in S05

These are the only newly surfaced truth-surface drifts I found during this scout pass.

#### C1. `mesher/ingestion/routes.mpl:299-302` overstates the bulk JSON limitation

**Current comment**

- `mesher/ingestion/routes.mpl:299-302`
  - says individual JSON array element parsing is not supported at the Mesh language level

**Evidence gathered**

1. **Nested list decoding works today**
   - Research-only repro: a wrapper struct with `items :: List < EventPayload >` and `deriving(Json)` compiled and ran successfully.
   - Observed output: `2`

2. **Bare top-level collection decoding is not directly exposed through `.from_json`**
   - Research-only repro: `pub type PayloadList = List < EventPayload >` then `PayloadList.from_json(json_str)` failed to compile.
   - Observed error: `undefined variable: PayloadList`

3. **Compiler surface supports collection decode under named derived types, not a bare collection type method**
   - `compiler/mesh-codegen/src/mir/lower.rs:6987-7034` has explicit `emit_collection_from_json(...)` logic for `List` / `Map`
   - `compiler/mesh-codegen/src/mir/lower.rs:7060-7062` says `StructName.from_json(str)` is implemented through a generated `__json_decode__StructName` wrapper
   - I did **not** find an equivalent top-level collection `.from_json` surface in parser/typechecker/codegen search

**Conclusion**

The current comment is **too broad**. Mesh can decode array elements when the array is nested inside a derived `Json` type. The real missing surface is the **bare top-level list decode** that this endpoint would need if it wanted to parse the request body directly as a raw JSON array.

**Recommended change shape**

Narrow the wording, for example:

- do **not** claim array parsing is absent from the language in general
- do say that this endpoint receives a **bare JSON array** and Mesher does not currently use or expose a direct top-level list `from_json` path here, so it stores the raw bulk JSON for downstream handling

This is the most important new S05 finding. If left unchanged, the final retained-limit ledger will preserve false folklore.

#### C2. `mesher/services/writer.mpl:61-62` keeps a stale “service dispatch codegen” rationale

**Current comment**

- `mesher/services/writer.mpl:61-62`
  - `# Kept as standalone functions so cast handler bodies remain minimal`
  - `# (avoids complex expressions inside service dispatch codegen).`

**Earlier context**

- `S03` intentionally did **not** touch this line because it was not part of the original named proof inventory and it sits near the real timer keep-site.
- See `.gsd/milestones/M032/slices/S03/S03-RESEARCH.md:47` and `:224`.

**Evidence gathered**

1. `S03` already proved direct `if/else` in cast handlers works:
   - `compiler/meshc/tests/e2e.rs:6868-6874` — `e2e_m032_supported_cast_if_else`

2. I ran a research-only inline repro that moved the same shape directly into a `cast Store(...) do|state| ... end` body:
   - append to list
   - compute `new_len`
   - branch for capped buffer
   - rebuild and return a new state struct
   - compile + run output: `2`

**Conclusion**

The “avoids complex expressions inside service dispatch codegen” rationale is **stale**.

The helper can still stay for readability / local reuse, but the comment should not present it as a current Mesh codegen limitation.

**Recommended change shape**

Either:

- remove the limitation rationale entirely, or
- rewrite it as a readability / local-reuse comment rather than a language/tooling constraint

Do **not** touch the real timer keep-site at `mesher/services/writer.mpl:153-154` while making this change.

### D. Wider ORM / migration pressure should be grouped and handed to M033

The final S05 artifact should keep this section **family-level**, not line-by-line.

#### D1. Current ORM boundary inventory is broad

`rg -n '^# ORM boundary:' mesher/storage/queries.mpl mesher/storage/writer.mpl` returned **16** explicit boundary comments.

Sampled families from `mesher/storage/queries.mpl` and `mesher/storage/writer.mpl`:

- **Computed upsert/update expressions**
  - `upsert_issue(...)` — `event_count = issues.event_count + 1`, `CASE` in `ON CONFLICT DO UPDATE`
- **Server-side JSONB extraction / insert paths**
  - `extract_event_fields(...)`
  - `insert_event(...)`
  - `create_alert_rule(...)`
  - `update_project_settings(...)`
- **Optional-filter + keyset pagination param-arity**
  - `list_issues_filtered(...)`
- **Parameterized SELECT expressions**
  - `search_events_fulltext(...)` (`ts_rank(...)` with bound query)
  - `event_breakdown_by_tag(...)` (`tags->>$2`)
- **Scalar / derived subquery composition**
  - `project_health_summary(...)`
  - `get_event_neighbors(...)`
  - `evaluate_threshold_rule(...)`
- **Function-valued SET expressions**
  - `acknowledge_alert(...)`
  - `resolve_fired_alert(...)`
- **Random / COALESCE / scalar-subquery sampling**
  - `check_sample_rate(...)`

These are honest follow-on pressures, but they are **not** S05 cleanup targets. They are exactly the API-expansion surface M033 is supposed to own.

#### D2. Migration / DDL boundary also remains real

**Mesher sites**

- `mesher/migrations/20260216120000_create_initial_schema.mpl:36-38`
  - `PARTITION BY RANGE` table creation is still raw SQL
- `mesher/storage/queries.mpl:939-954`
  - partition catalog lookup via `pg_inherits` / `pg_class`
  - raw `DROP TABLE IF EXISTS ...` partition cleanup
- `mesher/storage/schema.mpl`
  - runtime partition creation still sits outside a higher-level migration DSL

**Compiler / typed surface evidence**

- `compiler/mesh-typeck/src/infer.rs:2595-2602`
  - current `Migration.create_table` type is only `fn(PoolHandle, String, List<String>) -> Result<Int, String>`
- `compiler/meshc/tests/e2e.rs:5386-5458`
  - migration e2e coverage exercises the existing typed function signatures only
- `compiler/meshc/src/migrate.rs:485-505`
  - generated migration template shows `Migration.create_table(...)`, `add_column(...)`, `create_index(...)`, `execute(...)`, but no partition-clause surface

**Conclusion**

The migration comment about `PARTITION BY` is still truthful and should enter the **M033 handoff section**, not the stale-folklore cleanup section.

### E. Type-file `from_json` notes are still control comments, not limitation folklore

**Files**

- `mesher/types/event.mpl:55`
- `mesher/types/issue.mpl:14`

These comments describe row-shape / JSONB-string decoding reality, not a Mesh module-boundary limitation. S05 should keep excluding them from stale-folklore grep reconciliation.

## Task Seams

### Task A — Truth-surface cleanup for the two remaining overbroad comments

**Files**

- `mesher/ingestion/routes.mpl`
- `mesher/services/writer.mpl`

**Do**

- Narrow the bulk-array wording in `handle_bulk_authed(...)`
- Remove or rewrite the stale “service dispatch codegen” explanation above `writer_store(...)`
- Keep adjacent real keep-sites intact:
  - `mesher/ingestion/routes.mpl:2`
  - `mesher/services/writer.mpl:153-154`

**Optional but useful**

If the planner wants durable proof rather than research-only evidence, add one or two tiny tests / fixtures before editing these comments:

- nested-list `from_json` support under a derived wrapper type
- inline complex cast-body repro matching the `writer_store(...)` shape

### Task B — Integrated closeout proof + retained-limit ledger

**Files likely touched**

- `.gsd/milestones/M032/slices/S05/S05-SUMMARY.md`
- `.gsd/milestones/M032/slices/S05/S05-UAT.md` (if the planner wants a UAT-style closeout script like S03/S04)
- `.gsd/milestones/M032/M032-ROADMAP.md`
- `.gsd/PROJECT.md`
- `.gsd/REQUIREMENTS.md`
- `.gsd/KNOWLEDGE.md`
- possibly `.gsd/DECISIONS.md` only if the executor makes a real scoping/ledger decision that is worth preserving

**Do**

- rerun the integrated proof
- publish the final retained-limit ledger
- make the M033 handoff explicit and family-level
- mark S05 complete only after the proof and ledger are both truthful

## Verification Plan

### 1. Integrated closeout proof

Run the authoritative replay first:

```bash
bash scripts/verify-m032-s01.sh
```

Then cite the stable named controls in the final artifact:

```bash
cargo test -q -p meshc --test e2e m032_inferred -- --nocapture
cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture
```

### 2. Final Mesher baseline

If the executor wants explicit standalone confirmation beyond the replay script:

```bash
cargo run -q -p meshc -- fmt --check mesher
cargo run -q -p meshc -- build mesher
```

### 3. Comment-truth reconciliation greps

**Old stale-folklore phrases from S03/S04 should stay gone:**

```bash
! rg -n "query string parsing not available in Mesh|complex case expressions|parser limitation with if/else in cast handlers|cross-module from_json limitation|from_json limitation per decision \[88-02\]|Validation is done by the caller|caller is responsible for JSON parsing and field validation|services and inferred/polymorphic functions cannot be exported across modules|must stay in main\.mpl" mesher
```

**The two newly surfaced overbroad phrases should be removed or narrowed:**

```bash
! rg -n "not supported at the Mesh language level|complex expressions inside service dispatch codegen" mesher/ingestion/routes.mpl mesher/services/writer.mpl
```

**Real keep-sites that must remain visible:**

```bash
rg -n "HTTP routing does not support closures|avoids && codegen issue inside nested if blocks|Timer.send_after delivers raw bytes" mesher/ingestion/routes.mpl mesher/services/stream_manager.mpl mesher/services/writer.mpl mesher/ingestion/pipeline.mpl
rg -n "single-expression case arm constraint|single-expression case arms|case arm extraction" mesher/services/event_processor.mpl mesher/ingestion/fingerprint.mpl mesher/services/retention.mpl mesher/ingestion/pipeline.mpl mesher/api/team.mpl
rg -n "from_json" mesher/types/event.mpl mesher/types/issue.mpl
rg -n '^# ORM boundary:' mesher/storage/queries.mpl mesher/storage/writer.mpl
rg -n 'Migration DSL does not support PARTITION BY|pg_inherits|DDL operation' mesher/migrations/20260216120000_create_initial_schema.mpl mesher/storage/queries.mpl
```

## Risks / Fragility

- **Biggest planning risk:** letting S05 balloon into M033. The ORM / migration inventory is large and real, but S05 should summarize it by family and hand it off.
- **Biggest truth risk:** writing the final retained-limit ledger **before** fixing the two overbroad comments above.
- **Case-arm proof is weaker than the other retained limits.** If the executor wants a pure grep-less proof surface there, it needs a new small parser/CLI repro.
- **Do not over-correct the bulk comment.** The research supports “bare top-level list decode is not directly exposed here,” not “the bulk endpoint can now parse arbitrary raw JSON arrays directly with no design change.”
- **Do not touch the SQL / partition paths in S05.** They are truthful keep-sites and explicit M033 pressure, not cleanup debt.

## Current Baseline Observed During Research

These commands were rerun successfully from repo root during this scout pass:

```bash
bash scripts/verify-m032-s01.sh
cargo test -q -p meshc --test e2e m032_inferred -- --nocapture
cargo test -q -p meshc --test e2e_stdlib e2e_m032_route_closure_runtime_failure -- --nocapture
```

Observed result:

- `verify-m032-s01: ok`
- inferred proof stayed green (`2 passed`)
- closure-route runtime failure control stayed green (`1 passed`)

Additional research-only repros found the last two misleading comment surfaces:

- nested-list wrapper decode under `deriving(Json)` works
- bare top-level list `.from_json` surface is still missing
- inlined writer-style `cast` body compiles and runs cleanly

That is enough evidence to plan S05 as **small truth-surface cleanup + integrated artifact closeout**, not as another broad compiler or ORM repair slice.
