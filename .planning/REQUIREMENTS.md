# Requirements: Mesh

**Defined:** 2026-02-27
**Milestone:** v13.0 Language Completeness
**Core Value:** Expressive, readable concurrency — writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.

## v13.0 Requirements

Requirements for v13.0. Each maps to roadmap phases.

### PIPE — Multi-line pipe continuation

- [x] **PIPE-01**: User can format pipe chains across multiple lines by placing `|>` or `|N>` at the end of a line or start of a continuation line
- [x] **PIPE-02**: Multi-line pipe chains produce identical output to their single-line equivalents (same semantics, no regressions)

### ALIAS — Type aliases

- [ ] **ALIAS-01**: User can declare `type Alias = ExistingType` in any module
- [ ] **ALIAS-02**: User can use a type alias anywhere the aliased type is valid (function signatures, struct field types, let bindings)
- [ ] **ALIAS-03**: User can export type aliases with `pub type Alias = ExistingType` for cross-module use
- [ ] **ALIAS-04**: Compiler emits an error when a type alias references an undefined type

### TRYFROM — TryFrom/TryInto traits

- [ ] **TRYFROM-01**: User can implement `TryFrom<F>` for a type with `fn try_from(value: F) -> Result<Self, E>`
- [ ] **TRYFROM-02**: `TryInto<T>` is automatically derived for any type implementing `TryFrom<F>` (mirrors From/Into pattern from v7.0)
- [ ] **TRYFROM-03**: `?` operator works ergonomically with `try_from`/`try_into` for fallible conversions in `Result`-returning functions

### MAPCOL — Map.collect string keys

- [ ] **MAPCOL-01**: User can collect an iterator of `{String, V}` pairs into `Map<String, V>` via `.collect()`

### QUAL — Code quality

- [ ] **QUAL-01**: All 3 pre-existing compiler warnings resolved (clean `cargo build --all` output)
- [ ] **QUAL-02**: Middleware handler parameter type is inferred without requiring explicit `:: Request` annotation

### DOGFOOD — Mesher dogfooding

- [ ] **DOGFOOD-01**: Mesher source updated to use multi-line pipes where long chains benefit from line breaks
- [ ] **DOGFOOD-02**: Mesher uses type aliases where repeated type patterns benefit from named aliases

### DOCS — Documentation site

- [ ] **DOCS-01**: Documentation site updated with multi-line pipe syntax (examples, cheatsheet)
- [ ] **DOCS-02**: Documentation site updated with type alias declaration and usage
- [ ] **DOCS-03**: Documentation site updated with TryFrom/TryInto trait documentation

## Future Requirements

### Language features (deferred to v14.0+)

- **ALIAS-GEN-01**: User can declare generic type aliases (`type Pair<T> = {T, T}`)
- **TREESITTER-01**: Tree-sitter grammar for Mesh for better editor integration
- **HOMEBREW-01**: Homebrew formula for `brew install mesh`
- **INLAY-01**: LSP inlay hints for inferred types
- **SEMANTIC-01**: LSP semantic token support

## Out of Scope

| Feature | Reason |
|---------|--------|
| Generic type aliases | Higher complexity; non-generic aliases cover common use cases for v13.0 |
| Tree-sitter grammar | Tooling milestone, not language features; deferred |
| Homebrew packaging | Distribution milestone, not language features; deferred |
| TryFrom for built-in primitives | Implementation complexity; user-defined types sufficient for v13.0 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PIPE-01 | Phase 126 | Complete |
| PIPE-02 | Phase 126 | Complete |
| ALIAS-01 | Phase 127 | Pending |
| ALIAS-02 | Phase 127 | Pending |
| ALIAS-03 | Phase 127 | Pending |
| ALIAS-04 | Phase 127 | Pending |
| TRYFROM-01 | Phase 128 | Pending |
| TRYFROM-02 | Phase 128 | Pending |
| TRYFROM-03 | Phase 128 | Pending |
| MAPCOL-01 | Phase 129 | Pending |
| QUAL-01 | Phase 129 | Pending |
| QUAL-02 | Phase 129 | Pending |
| DOGFOOD-01 | Phase 130 | Pending |
| DOGFOOD-02 | Phase 130 | Pending |
| DOCS-01 | Phase 131 | Pending |
| DOCS-02 | Phase 131 | Pending |
| DOCS-03 | Phase 131 | Pending |

**Coverage:**
- v13.0 requirements: 17 total
- Mapped to phases: 17
- Unmapped: 0 ✓

---
*Requirements defined: 2026-02-27*
*Last updated: 2026-02-27 after roadmap creation*
