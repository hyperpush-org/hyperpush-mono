# S02: Cross-module and inferred-export blocker retirement

**Goal:** Fix the real lowering bug behind unconstrained inferred exports so local and cross-module call sites both produce the right ABI and runtime values, then dogfood that repaired path in mesher by importing a real inferred-parameter helper from `Storage.Writer` instead of keeping stale workaround lore.
**Demo:** `compiler/meshc/tests/e2e.rs` contains passing `m032_inferred_*` coverage for local and cross-module inferred exports, `scripts/verify-m032-s01.sh` no longer treats `xmod_identity` as a retained failure, and `mesher/services/writer.mpl` builds against an inferred-parameter export from `mesher/storage/writer.mpl` while `cargo run -q -p meshc -- fmt --check mesher` and `cargo run -q -p meshc -- build mesher` stay green.

## Must-Haves

- The compiler fix repairs the root cause in MIR lowering: unresolved inferred function parameters and returns recover concrete types from real usage on both local and imported definitions instead of degrading to unit-like ABI.
- `compiler/meshc/tests/e2e.rs` gains passing M032 regression coverage for local inferred identity, cross-module inferred identity, and adjacent already-green cross-module controls so the slice proves the repaired path without widening into speculative generic work.
- Mesher dogfoods the repaired path by moving a real inferred-parameter batch-write helper into `mesher/storage/writer.mpl`, importing it from `mesher/services/writer.mpl`, and removing the stale `main.mpl` / service-export wording from `mesher/storage/writer.mpl` while preserving the honest raw-SQL boundary rationale.
- `scripts/verify-m032-s01.sh` and the mesher fmt/build gates reflect current truth: `xmod_identity` is now a supported path, while the other retained-limit checks remain unchanged.

## Proof Level

- This slice proves: integration
- Real runtime required: yes
- Human/UAT required: no

## Verification

- `cargo test -p meshc --test e2e m032_inferred -- --nocapture`
- `cargo test -p meshc --test e2e m032_inferred_cross_module_identity -- --nocapture`
- `cargo test -p meshc --test e2e e2e_cross_module_polymorphic -- --nocapture`
- `cargo test -p meshc --test e2e e2e_cross_module_service -- --nocapture`
- `bash scripts/verify-m032-s01.sh`
- `rg -n "^pub fn flush_batch|flush_batch\(" mesher/storage/writer.mpl mesher/services/writer.mpl`
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- build mesher`

## Observability / Diagnostics

- Runtime signals: named `m032_inferred_*` test failures, wrong-stdout diffs for local/cross-module identity runs, and `verify-m032-s01.sh` step names when the repaired path drifts back to LLVM verifier errors.
- Inspection surfaces: `compiler/meshc/tests/e2e.rs`, `scripts/verify-m032-s01.sh`, `mesher/storage/writer.mpl`, `mesher/services/writer.mpl`, and the direct `meshc build mesher` CLI path.
- Failure visibility: the slice must leave failures attributable to a specific surface (`local inferred return`, `imported inferred export`, or `mesher dogfood import`) rather than a generic verifier crash with no fixture name.
- Redaction constraints: none; keep proofs local to repo fixtures and compiler/mesher commands.

## Integration Closure

- Upstream surfaces consumed: `compiler/meshc/src/main.rs`, `compiler/mesh-codegen/src/lib.rs`, `compiler/mesh-codegen/src/mir/lower.rs`, `compiler/mesh-codegen/src/mir/types.rs`, `compiler/meshc/tests/e2e.rs`, `mesher/storage/writer.mpl`, `mesher/services/writer.mpl`, `scripts/verify-m032-s01.sh`
- New wiring introduced in this slice: cross-module function-usage evidence flows from the `meshc` driver into MIR lowering, and `Services.Writer` imports a real inferred-parameter helper from `Storage.Writer`.
- What remains before the milestone is truly usable end-to-end: S03 still needs stale request/handler cleanup, S04 still needs module-boundary `from_json` convergence, and S05 still needs the final retained-limit ledger plus integrated closeout.

## Tasks

- [x] **T01: Repair inferred-export lowering and freeze regression coverage** `est:3h`
  - Why: The slice only counts if it fixes the actual ABI/root-cause bug rather than swapping one verifier symptom for another. The repair needs durable proof on both the local single-file path and the imported cross-module path.
  - Files: `compiler/meshc/tests/e2e.rs`, `compiler/meshc/src/main.rs`, `compiler/mesh-codegen/src/lib.rs`, `compiler/mesh-codegen/src/mir/lower.rs`, `compiler/mesh-codegen/src/mir/types.rs`
  - Do: Replace the old `xmod_identity` failure-only proof with passing `m032_inferred_*` coverage in `compiler/meshc/tests/e2e.rs`: local inferred identity must round-trip both `Int` and `String`, imported inferred identity must do the same through a real multifile build, and the tests must stay narrow enough that `e2e_cross_module_polymorphic` / `e2e_cross_module_service` remain the adjacency controls instead of being rewritten. Then thread concrete function-usage evidence from `compiler/meshc/src/main.rs` through `mesh_codegen::lower_to_mir_raw(...)` into the MIR lowerer so imported functions can recover unresolved parameter and return types from call-site evidence; extend `lower_fn_def(...)` to repair unresolved returns as well as parameters, and do not hide real unresolved types with verifier suppression or broad generic machinery.
  - Verify: `cargo test -p meshc --test e2e m032_inferred -- --nocapture`; `cargo test -p meshc --test e2e e2e_cross_module_polymorphic -- --nocapture`; `cargo test -p meshc --test e2e e2e_cross_module_service -- --nocapture`
  - Done when: the new `m032_inferred_*` tests pass with correct stdout on both local and imported fixtures, and the existing cross-module service/polymorphic controls still pass unchanged.
- [ ] **T02: Dogfood the repaired inferred export in mesher and replay automation** `est:2h`
  - Why: R013 and R035 are not satisfied by compiler tests alone. Mesher needs to consume the repaired path in real product code, and the repo’s public proof surface has to stop claiming the blocker is still live.
  - Files: `mesher/storage/writer.mpl`, `mesher/services/writer.mpl`, `scripts/verify-m032-s01.sh`
  - Do: Move `flush_batch` and any storage-local helper it needs out of `mesher/services/writer.mpl` into `mesher/storage/writer.mpl` so `Services.Writer` imports and calls a real inferred-parameter export instead of keeping all inferred helpers local. Keep retry policy, service state, and timer logic where they are; this task is about truthful module boundaries, not redesigning the writer service. Rewrite the stale `main.mpl` / service-export comment in `mesher/storage/writer.mpl` around the actual remaining raw-SQL boundary, then flip `scripts/verify-m032-s01.sh` so `xmod_identity` is a success path with exact stdout while the still-real retained-limit checks stay intact.
  - Verify: `rg -n "^pub fn flush_batch|flush_batch\(" mesher/storage/writer.mpl mesher/services/writer.mpl`; `bash scripts/verify-m032-s01.sh`; `cargo run -q -p meshc -- fmt --check mesher`; `cargo run -q -p meshc -- build mesher`
  - Done when: mesher imports and uses the inferred `flush_batch` export successfully, `mesher/storage/writer.mpl` no longer claims services or inferred exports must stay in `main.mpl`, and the replay script/build gates all pass with `xmod_identity` treated as supported behavior.

## Files Likely Touched

- `compiler/meshc/tests/e2e.rs`
- `compiler/meshc/src/main.rs`
- `compiler/mesh-codegen/src/lib.rs`
- `compiler/mesh-codegen/src/mir/lower.rs`
- `compiler/mesh-codegen/src/mir/types.rs`
- `mesher/storage/writer.mpl`
- `mesher/services/writer.mpl`
- `scripts/verify-m032-s01.sh`
