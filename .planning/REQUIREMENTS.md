# Requirements: Mesh

**Defined:** 2026-02-25
**Core Value:** Expressive, readable concurrency — writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.

## v12.0 Requirements

Requirements for v12.0 Language Ergonomics & Open Source Readiness. Each maps to roadmap phases.

### Slot Pipe Operator

- [x] **PIPE-01**: User can write `expr |2> func(a)` to pipe result as second argument (becomes `func(a, expr)`)
- [x] **PIPE-02**: User can write `expr |N>` for any argument position N ≥ 2
- [x] **PIPE-03**: Slot pipes are chainable: `a |2> f(b) |> g()` works correctly
- [x] **PIPE-04**: Type inference validates slot pipe position against function arity with a clear error
- [ ] **PIPE-05**: Mesher codebase updated using slot pipe where it improves readability (dogfooding verified)

### Regular Expressions

- [ ] **REGEX-01**: User can write regex literals `~r/pattern/` and `~r/pattern/flags` (i, m, s flags)
- [ ] **REGEX-02**: User can compile regex at runtime: `Regex.compile(str) -> Result<Regex, String>`
- [ ] **REGEX-03**: User can test a match: `Regex.match(rx, str) -> Bool`
- [ ] **REGEX-04**: User can extract captures: `Regex.captures(rx, str) -> Option<List<String>>`
- [ ] **REGEX-05**: User can replace matches: `Regex.replace(rx, str, replacement) -> String`
- [ ] **REGEX-06**: User can split by pattern: `Regex.split(rx, str) -> List<String>`

### String Ergonomics

- [x] **STRG-01**: User can write string interpolation `"Value: #{expr}"` supporting arbitrary expressions
- [ ] **STRG-02**: User can write heredoc strings `"""..."""` for multiline content without escape sequences
- [ ] **STRG-03**: Heredoc strings support interpolation: `"""{"id": "#{id}"}"""`
- [ ] **STRG-04**: User can read env var with default: `Env.get("KEY", "default") -> String`
- [ ] **STRG-05**: User can parse env var as integer with default: `Env.get_int("PORT", 8080) -> Int`
- [ ] **STRG-06**: Mesher server code updated using new string features (dogfooding verified)

### Mesh Agent Skill

- [ ] **SKILL-01**: Mesh language agent skill created in GSD skill format with progressive disclosure
- [ ] **SKILL-02**: Skill has a main entry command providing language overview and available sub-topics
- [ ] **SKILL-03**: Skill has per-topic deep-dive commands (syntax, types, actors, ORM, HTTP/WS, stdlib, distributed actors)
- [ ] **SKILL-04**: Skill registered and usable by AI for all Mesh-related questions without explicit invocation

### Repository Reorganization

- [ ] **REPO-01**: Compiler Rust crates moved under `compiler/` directory
- [ ] **REPO-02**: Mesher application moved under `mesher/` directory
- [ ] **REPO-03**: Documentation website moved under `website/` directory
- [ ] **REPO-04**: Install scripts and build tooling moved under `tools/` directory
- [ ] **REPO-05**: All CI/CD pipelines (GitHub Actions) updated for new directory structure
- [ ] **REPO-06**: All tests pass and Mesher E2E verified after reorganization

### Performance Benchmarks

- [ ] **BENCH-01**: Mesh benchmark HTTP server written (JSON endpoint, configurable concurrency)
- [ ] **BENCH-02**: Equivalent benchmark server written in Go (net/http or Gin)
- [ ] **BENCH-03**: Equivalent benchmark server written in Rust (axum or actix-web)
- [ ] **BENCH-04**: Equivalent benchmark server written in Elixir (Plug/Cowboy)
- [ ] **BENCH-05**: Benchmarks measure throughput (req/s), p50/p99 latency, and memory usage
- [ ] **BENCH-06**: Methodology documented (tool, hardware, concurrency settings) and results published in repo

## Future Requirements

### Language Features (v13.0+)

- Multi-line pipe continuation (parser support for `|>` at start of next line)
- TryFrom/TryInto traits (fallible conversion)
- Infinite iterators and iterator fusion optimization
- Type aliases

### Tooling (v13.0+)

- Semantic tokens for LSP
- Workspace symbols
- Tree-sitter grammar
- Homebrew packaging
- Inlay hints

## Out of Scope

| Feature | Reason |
|---------|--------|
| Hot code reloading | Complex runtime integration, not blocking v12.0 goals |
| Distributed ETS / process groups | Runtime feature, separate milestone when needed |
| Atom cache optimization | Performance micro-optimization, not urgent |
| Browser playground (WASM) | Large effort, website milestone |
| Generational GC | Mark-sweep is sufficient, premature optimization |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PIPE-01 | Phase 116 | Complete |
| PIPE-02 | Phase 116 | Complete |
| PIPE-03 | Phase 116 | Complete |
| PIPE-04 | Phase 116 | Complete |
| PIPE-05 | Phase 120 | Pending |
| REGEX-01 | Phase 119 | Pending |
| REGEX-02 | Phase 119 | Pending |
| REGEX-03 | Phase 119 | Pending |
| REGEX-04 | Phase 119 | Pending |
| REGEX-05 | Phase 119 | Pending |
| REGEX-06 | Phase 119 | Pending |
| STRG-01 | Phase 117 | Complete |
| STRG-02 | Phase 117 | Pending |
| STRG-03 | Phase 117 | Pending |
| STRG-04 | Phase 118 | Pending |
| STRG-05 | Phase 118 | Pending |
| STRG-06 | Phase 120 | Pending |
| SKILL-01 | Phase 121 | Pending |
| SKILL-02 | Phase 121 | Pending |
| SKILL-03 | Phase 121 | Pending |
| SKILL-04 | Phase 121 | Pending |
| REPO-01 | Phase 122 | Pending |
| REPO-02 | Phase 122 | Pending |
| REPO-03 | Phase 122 | Pending |
| REPO-04 | Phase 122 | Pending |
| REPO-05 | Phase 122 | Pending |
| REPO-06 | Phase 122 | Pending |
| BENCH-01 | Phase 123 | Pending |
| BENCH-02 | Phase 123 | Pending |
| BENCH-03 | Phase 123 | Pending |
| BENCH-04 | Phase 123 | Pending |
| BENCH-05 | Phase 123 | Pending |
| BENCH-06 | Phase 123 | Pending |

**Coverage:**
- v12.0 requirements: 33 total
- Mapped to phases: 33
- Unmapped: 0 ✓

---
*Requirements defined: 2026-02-25*
*Last updated: 2026-02-25 after v12.0 roadmap creation (phases 116-123)*
