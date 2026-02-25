# Project Milestones: Mesh

## v7.0 Iterator Protocol & Trait Ecosystem (Shipped: 2026-02-14)

**Delivered:** Added associated types to the trait system and built a comprehensive trait-based protocol ecosystem: lazy iterators with pipe-style composition, From/Into conversion with ? operator error type conversion, numeric traits for user-extensible arithmetic, Collect for iterator materialization, and complete website documentation for all new features.

**Phases completed:** 74-80 (17 plans total)

**Key accomplishments:**
- Associated types in traits (`type Item`, `Self.Item`) with full HM inference integration and clear error diagnostics
- User-extensible arithmetic operators via Add/Sub/Mul/Div/Neg numeric traits with Output associated type
- Iterator/Iterable two-trait protocol with built-in implementations for List, Map, Set, Range and for-in desugaring
- From/Into conversion traits with automatic Into generation, built-in primitive conversions, and ? operator error type conversion
- Lazy iterator pipeline composition (map, filter, take, skip, enumerate, zip) with type-tag dispatch and short-circuit evaluation
- Terminal operations (count, sum, any, all, find, reduce) and Collect trait for materialization into List, Map, Set, String
- Complete website documentation: new Iterators page, updated Type System, Cheatsheet, and Language Basics pages

**Stats:**
- 128 files changed, +16,117 / -334 lines
- 97,190 lines of Rust (+3,675 from v6.0)
- 7 phases, 17 plans
- 2 days (2026-02-13 -> 2026-02-14)
- 84 commits
- 33/33 requirements satisfied

**Git range:** `feat(74-01)` -> `docs(phase-80)`

**What's next:** All 17 milestones complete. Potential directions include additional combinators (flat_map, chain, skip_while, take_while), bounded associated types, TryFrom/TryInto, infinite iterators, iterator fusion optimization, or new language features.

---

## v6.0 Website & Documentation (Shipped: 2026-02-13)

**Delivered:** Complete documentation website and landing page for the Mesh programming language, built with VitePress, Tailwind v4, and shadcn-vue, featuring custom Mesh syntax highlighting, monochrome design with dark/light mode, 9 documentation guides covering all language features, and production-quality site features (search, SEO, copy button, 404 page, edit links, version badge).

**Phases completed:** 70-73 (11 plans total)

**Key accomplishments:**
- VitePress custom theme with Tailwind v4 + shadcn-vue, monochrome OKLCH palette, dark/light mode toggle with FOUC prevention
- Custom Mesh syntax highlighting via TextMate grammar with dual monochrome Shiki themes (mesh-light/mesh-dark)
- Landing page with hero section, 4-feature showcase (actors, pattern matching, type inference, pipe operator), and "Why Mesh?" comparison vs Elixir/Rust/Go
- Full documentation: Getting Started, Language Basics, Type System, Concurrency, Web, Databases, Distributed Actors, Tooling, Syntax Cheatsheet
- Docs infrastructure: collapsible sidebar, table of contents, prev/next links, mobile-responsive Sheet-based sidebar
- Site features: MiniSearch full-text search, copy-to-clipboard, SEO meta tags, custom 404, edit-on-GitHub links, git-based last-updated timestamps, version badge

**Stats:**
- 108 files changed, 15,197 insertions
- 5,134 LOC website source (Vue, TypeScript, CSS, Markdown)
- 93,515 lines of Rust (unchanged -- website milestone)
- 4 phases, 11 plans
- 1 day (2026-02-13)
- 32 commits
- 32/32 requirements satisfied

**Git range:** `feat(70-01)` → `feat(73-03)`

**What's next:** All 16 milestones complete. Language, compiler, runtime, stdlib, database drivers, HTTP/WebSocket servers, distributed actors, and documentation site all shipped. Potential directions include browser playground (WASM), auto-generated API docs, community building, or v7.0 features.

---

## v5.0 Distributed Actors (Shipped: 2026-02-13)

**Delivered:** BEAM-style distributed actor system enabling Snow programs on different machines to form a cluster with location-transparent PIDs, remote spawn, cross-node monitoring, global process registry, and a binary wire format (STF) over TLS -- all with zero new crate dependencies.

**Phases completed:** 63-69 (20 plans total)

**Key accomplishments:**
- Location-transparent PIDs with 16-bit node_id bit-packing in existing u64 and Snow Term Format (STF) binary serialization for all Snow types with round-trip fidelity
- TLS-encrypted inter-node connections with HMAC-SHA256 cookie-based authentication, ephemeral ECDSA P-256 certificates, heartbeat dead connection detection, and automatic mesh formation
- Transparent remote send: `send(pid, msg)` routes to remote nodes automatically; `send({name, node}, msg)` for named remote processes; message ordering preserved per sender-receiver pair
- Distributed fault tolerance: remote process monitors (`:down`), node monitors (`:nodedown`/`:nodeup`), bidirectional exit signal propagation, and partition-aware link cleanup
- Remote spawn via function name registry: `Node.spawn(node, function, args)` and `Node.spawn_link` with LLVM codegen for Node/Process/Global modules (10+ new intrinsics)
- Global process registry: `Global.register/whereis/unregister` with cluster-wide broadcast, sync-on-connect, and automatic cleanup on node disconnect
- Cross-node integration: WebSocket room broadcasts transparently reach all cluster nodes via DIST_ROOM_BROADCAST; supervision trees can monitor and restart remote children

**Stats:**
- 75 files modified
- 93,515 lines of Rust (+9,115 net from v4.0)
- 7 phases, 20 plans
- 1 day (2026-02-12 -> 2026-02-13)
- 75 commits
- 29/29 requirements satisfied

**Git range:** `feat(63-01)` -> `docs(phase-69)`

**What's next:** TBD -- distributed actors complete. Potential directions include atom cache optimization, process groups (pg), distributed ETS, hot code reloading, iterator protocol, or incremental compilation.

---

## v4.0 WebSocket Support (Shipped: 2026-02-13)

**Delivered:** Added real-time bidirectional communication to Snow with a complete RFC 6455 WebSocket implementation: frame codec with masking, HTTP upgrade handshake, actor-per-connection server with crash isolation, TLS (wss://), heartbeat with dead connection detection, message fragmentation, and named rooms with pub/sub broadcast.

**Phases completed:** 59-62 (8 plans total)

**Key accomplishments:**
- RFC 6455 frame codec with 3 payload length encodings, XOR masking, 6 opcodes, HTTP upgrade handshake with Sec-WebSocket-Accept, close handshake, and UTF-8 text validation
- Actor-per-connection WebSocket server with reader thread bridge delivering frames to actor mailbox via reserved type tags, crash isolation (close 1011), and exit signal propagation
- Callback API (on_connect/on_message/on_close) with connection rejection, Ws.serve(handler, port) entry point, Ws.send/Ws.send_binary for responses
- TLS (wss://) via WsStream enum reusing existing rustls 0.23 infrastructure, with build_server_config shared across HTTP and WS
- Heartbeat ping/pong with 30s interval, 10s dead connection timeout, payload validation, plus fragment reassembly with 16 MiB limit and interleaved control frame handling
- Named rooms with RoomRegistry (join/leave/broadcast/broadcast_except), concurrent access via dual-map design, and automatic disconnect cleanup

**Stats:**
- 47 files modified
- ~84,400 lines of Rust (+~950 net from v3.0)
- 4 phases, 8 plans, 50 truths verified
- 1 day (2026-02-12)
- 38 commits
- 37/37 requirements satisfied, 1,524 tests passing (+44 new WebSocket-specific tests)

**Git range:** `feat(59-01)` -> `feat(62-02)`

---

## v3.0 Production Backend (Shipped: 2026-02-12)

**Delivered:** Made Snow viable for production backend deployments with TLS encryption for PostgreSQL and HTTPS, connection pooling with health checks, panic-safe database transactions, and automatic struct-to-row mapping via `deriving(Row)`.

**Phases completed:** 55-58 (8 plans total)

**Key accomplishments:**
- PostgreSQL TLS via SSLRequest protocol with sslmode=disable/prefer/require, PgStream enum for zero-cost Plain/Tls dispatch, rustls 0.23 with webpki-roots
- HTTPS server via Http.serve_tls with hand-rolled HTTP/1.1 parser replacing tiny_http, HttpStream enum mirroring PgStream pattern
- Connection pooling with Mutex+Condvar synchronization, configurable min/max/timeout, health check (SELECT 1) on checkout, automatic ROLLBACK on dirty checkin
- Database transactions: Pg.begin/commit/rollback/transaction with catch_unwind panic-safe rollback, Sqlite.begin/commit/rollback manual control
- Struct-to-row mapping via deriving(Row) with from_row function generation, Pg.query_as/Pool.query_as one-step query+hydration, E0039 compile error for non-mappable fields

**Stats:**
- 48 files modified
- 83,451 lines of Rust (+2,445 net from v2.0)
- 4 phases, 8 plans
- 1 day (2026-02-12)
- 33 commits
- 24/24 requirements satisfied, 290+ tests passing (+18 new v3.0 tests)

**Git range:** `feat(55-01)` -> `docs(v3.0)`

---

## v2.0 Database & Serialization (Shipped: 2026-02-12)

**Delivered:** Made Snow viable for real backend applications with JSON struct/sum-type serde via `deriving(Json)`, SQLite and PostgreSQL database drivers with parameterized queries, HTTP path parameters with method-specific routing, and composable middleware pipelines.

**Phases completed:** 49-54 (13 plans total)

**Key accomplishments:**
- JSON struct serde with `deriving(Json)`, typed extractors, and callback-based collection encode/decode for nested structs, Option, List, Map fields
- Sum type and generic struct JSON serialization via tagged union encoding (`{"tag":"V","fields":[...]}`) and monomorphization
- HTTP path parameter matching (`/users/:id`) with method-specific routing (GET/POST/PUT/DELETE) and three-pass priority (exact > parameterized > wildcard)
- Composable middleware pipeline with trampoline-based chain execution, short-circuit support, and automatic 404 wrapping
- SQLite driver with bundled C FFI (zero system deps), opaque GC-safe u64 handles, and `?` parameterized queries
- PostgreSQL pure wire protocol client (~550 lines Rust) with SCRAM-SHA-256/MD5 auth and Extended Query protocol ($1, $2 params)

**Stats:**
- 76 files modified
- 81,006 lines of Rust (+4,906 net from v1.9)
- 6 phases, 13 plans
- 2 days (2026-02-11 -> 2026-02-12)
- 52 commits
- 32/32 requirements satisfied, 287+ tests passing (+16 new v2.0 E2E tests)

**Git range:** `feat(49-01)` -> `docs(54-02)`

---

## v1.9 Stdlib & Ergonomics (Shipped: 2026-02-10)

**Delivered:** Made Snow practical for real programs by adding math stdlib via LLVM intrinsics, ? operator for Result/Option error propagation, receive timeouts and timer primitives, 20 collection operations across List/Map/Set/String, and self-recursive tail-call elimination -- all with zero new Rust crate dependencies and zero regressions.

**Phases completed:** 43-48 (13 plans total)

**Key accomplishments:**
- Math stdlib: 10 numeric operations (abs, min, max, pow, sqrt, floor, ceil, round, pi) via LLVM intrinsics + bidirectional Int/Float type conversion
- ? operator: Result<T,E> and Option<T> error propagation desugared to Match+Return in MIR, with E0036/E0037 diagnostics for misuse
- Receive timeouts: `receive { ... } after ms -> body` with null-check branching codegen completing the actor timeout feature
- Timer primitives: Timer.sleep (cooperative yield loop) and Timer.send_after (background OS thread with deep-copied message)
- Collection operations: 20 functions across List (sort, find, any, all, contains, zip, flat_map, flatten, enumerate, take, drop), String (split, join, to_int, to_float), Map (merge, to_list, from_list), Set (difference, to_list, from_list)
- Tail-call elimination: Self-recursive tail calls transformed to loops via MIR rewrite pass, supporting 7 tail position contexts (if/else, case, receive, blocks, let-chains, multi-clause, actor), 1M+ iterations without stack overflow

**Stats:**
- 89 files modified
- 76,100 lines of Rust (+2,716 net from v1.8)
- 6 phases, 13 plans
- 2 days (2026-02-09 -> 2026-02-10)
- 56 commits
- 28/28 requirements satisfied, 1,419 tests passing (+37 new v1.9-specific e2e tests)

**Git range:** `feat(43-01)` -> `docs(48-02)`

---

## v1.8 Module System (Shipped: 2026-02-09)

**Delivered:** Complete module system enabling multi-file projects with file-based modules, pub visibility, qualified and selective imports, dependency graph resolution with cycle detection, cross-module type checking for functions/structs/sum types/traits, module-qualified name mangling, and module-aware diagnostics -- compiled into a single LLVM binary via MIR merge.

**Phases completed:** 37-42 (12 plans total)

**Key accomplishments:**
- File-based module graph with recursive `.snow` discovery, path-to-name convention (`math/vector.snow` -> `Math.Vector`), Kahn's toposort, and cycle detection
- Multi-file build pipeline (`snowc build <dir>`) with ProjectData, per-module parsing, and zero single-file regressions across 84 pre-existing E2E tests
- Cross-module type checking with qualified imports (`import M` -> `M.fn()`), selective imports (`from M import { fn }`), accumulator-pattern inference, and MIR merge codegen
- Private-by-default visibility with `pub` modifier, PrivateItem E0035 error with "add pub" suggestion
- Module-qualified name mangling (`ModuleName__fn`) preventing private name collisions, cross-module generic monomorphization
- Module-aware diagnostics: file paths in errors via ariadne named-sources, module-qualified type names in type errors (`Geometry.Point`)

**Stats:**
- 70 files modified
- 73,384 lines of Rust (+2,883 net from v1.7)
- 6 phases, 12 plans
- 4 days (2026-02-05 -> 2026-02-09)
- 52 commits
- 27/27 requirements satisfied, 111 E2E tests passing (+31 new in v1.8)

**Git range:** `feat(37-01)` -> `docs(phase-42)`

---

## v1.7 Loops & Iteration (Shipped: 2026-02-09)

**Delivered:** Complete loop and iteration system with while loops, for-in over ranges and collections (List, Map, Set), break/continue control flow, comprehension semantics returning collected lists, filter clause (`when`), and actor-safe reduction checks at loop back-edges.

**Phases completed:** 33-36 (8 plans total)

**Key accomplishments:**
- While loops (`while condition do body end`) with break/continue, loop-depth tracking, closure boundary enforcement (E0032/E0033)
- For-in over integer ranges (`for i in 0..10 do body end`) with zero-allocation counter and half-open range semantics
- For-in over collections (List, Map with `{k,v}` destructuring, Set) with indexed iteration and O(N) list builder
- Comprehension semantics: all for-in loops return `List<T>` of collected body results; break returns partial list
- Filter clause (`for x in list when condition do body end`) with five-block codegen pattern across all collection types
- Reduction checks at loop back-edges for actor scheduler fairness; runtime list builder for O(N) allocation

**Stats:**
- 53 files modified
- 70,501 lines of Rust (+2,955 net from v1.6)
- 4 phases, 8 plans
- 2 days (2026-02-08 → 2026-02-09)
- 34 commits

**Git range:** `feat(33-01)` → `docs(phase-36)`

---

## v1.6 Method Dot-Syntax (Shipped: 2026-02-09)

**Delivered:** Method dot-syntax (`value.method(args)`) with automatic self-parameter desugaring, working across struct, primitive, generic, and collection types, with true chaining, mixed field/method access, deterministic ambiguity diagnostics, and zero regressions across all existing syntax forms.

**Phases completed:** 30-32 (6 plans total)

**Key accomplishments:**
- Method dot-syntax (`value.method(args)`) with automatic self-parameter desugaring via retry-based resolution in type checker and shared resolve_trait_callee in MIR
- Primitive, generic, and collection types all callable via dot syntax (`42.to_string()`, `my_list.to_string()`)
- True method chaining (`p.to_string().length()`) and mixed field/method access (`p.name.length()`)
- Deterministic alphabetical ambiguity diagnostics with per-trait qualified syntax suggestions
- Stdlib module method fallback enabling dot-syntax for String, List, Map, Set, Range module functions
- Zero regressions across all 5 existing syntax forms (struct fields, module-qualified, pipes, sum types, actors)

**Stats:**
- 28 files modified
- 67,546 lines of Rust (+1,025 net from v1.5)
- 3 phases, 6 plans, 12 tasks
- 1 day (2026-02-08 → 2026-02-09)
- 24 commits
- 1,255 tests passing (+23 new in v1.6)

**Git range:** `feat(30-01)` → `feat(32-01)`

**What's next:** TBD -- method dot-syntax complete. Potential directions include inherent methods (impl without trait), method references, associated types, Iterator/From protocols, distributed actors, or hot code reloading.

---

## v1.5 Compiler Correctness (Shipped: 2026-02-09)

**Delivered:** Resolved all three remaining known limitations -- polymorphic List<T>, Ord-requires-Eq compile-time enforcement, and higher-order constraint propagation -- making the Snow type and trait systems fully correct with zero known compiler correctness issues.

**Phases completed:** 26-29 (6 plans total)

**Key accomplishments:**
- Polymorphic List<T> with any element type (String, Bool, structs, nested lists) via ListLit MIR + snow_list_from_array codegen
- List trait integration: callback-based Display/Debug/Eq/Ord dispatch for List<T>
- Cons pattern destructuring (head :: tail) for all list element types with ListDecons decision tree
- Compile-time trait deriving safety: E0029 error when deriving Ord without Eq, with suggestion
- Qualified types: trait constraints propagate through higher-order function arguments (apply(show, 42))

**Stats:**
- 54 files modified
- 66,521 lines of Rust (+1,973 net from v1.4)
- 4 phases, 6 plans
- 1 day (2026-02-08 → 2026-02-09)
- 29 commits
- 1,232 tests passing (+26 new in v1.5)

**Git range:** `feat(26-01)` → `test(29-01)`

**What's next:** TBD -- all compiler correctness issues resolved. Zero known limitations. Potential directions include Iterator/From protocols (associated types), method dot-syntax, blanket impls, distributed actors, or hot code reloading.

---

## v1.4 Compiler Polish (Shipped: 2026-02-08)

**Delivered:** Fixed all five known limitations from v1.3 -- pattern matching codegen, Ordering type, nested collection Display, generic type deriving, and higher-order constraint soundness -- making the compiler fully correct across its type and trait systems.

**Phases completed:** 23-25 (5 plans total)

**Key accomplishments:**
- Sum type pattern matching codegen fixed: constructor field extraction via sum_type_defs threading through compile_match pipeline
- Ordering (Less|Equal|Greater) registered as built-in sum type with compare() dispatching via Ord trait
- Recursive nested collection Display with synthetic MIR wrapper functions for callback bridging
- Generic type auto-derive with parametric trait impl registration and lazy monomorphization at struct literal sites
- Type system soundness: where-clause constraints propagate through let-binding aliases, preventing unsound calls

**Stats:**
- 28 files modified
- 64,548 lines of Rust (+1,359 net from v1.3)
- 3 phases, 5 plans, ~10 tasks
- 1 day (2026-02-08)
- 13 commits
- 1,206 tests passing (+19 new in v1.4)

**Git range:** `feat(23-01)` → `test(25-01)`

**What's next:** TBD -- all v1.x compiler correctness issues resolved. Potential directions include Iterator/From protocols (requires associated types), method dot-syntax, blanket impls, distributed actors, or hot code reloading.

---

## v1.3 Traits & Protocols (Shipped: 2026-02-08)

**Delivered:** Complete trait/protocol system with user-defined interfaces, impl blocks, static dispatch via monomorphization, and six stdlib protocols (Display, Debug, Eq, Ord, Hash, Default) plus auto-derive support.

**Phases completed:** 18-22 (18 plans total)

**Key accomplishments:**
- Trait infrastructure: structural type matching via temporary unification, replacing string-based type_to_key
- Trait method codegen: ImplDef lowering to MIR with Trait__Method__Type mangled names and static dispatch
- Essential stdlib protocols: Display, Debug, Eq, Ord with string interpolation integration and auto-derived for all non-generic types
- Extended protocols: Hash (FNV-1a), Default (static methods), default method implementations, collection Display/Debug
- Auto-derive system: `deriving(Eq, Ord, Display, Debug, Hash)` with conditional gating and backward compatibility

**Stats:**
- 77 files modified
- 63,189 lines of Rust (+5,532 net from v1.2)
- 5 phases, 18 plans
- 1 day (2026-02-07 → 2026-02-08)
- 65 commits
- 1,187 tests passing (+130 new in v1.3)

**Git range:** `feat(18-01)` → `feat(22-02)`

**What's next:** TBD -- trait system complete. Potential directions include Iterator/From protocols, method dot-syntax, blanket impls, distributed actors, or hot code reloading.

---

## v1.2 Runtime & Type Fixes (Shipped: 2026-02-08)

**Delivered:** Fun() type annotation parsing and mark-sweep garbage collector for per-actor heaps, fixing the two remaining known issues from v1.1.

**Phases completed:** 16-17 (6 plans total)

**Key accomplishments:**
- Fun() type annotations fully integrated: parser (FUN_TYPE CST node) through type checker (Ty::Fun) to codegen (MirType::Closure)
- Mark-sweep GC with 16-byte GcHeader, conservative stack scanning, and worklist-based tricolor marking
- Per-actor cooperative GC at yield points -- no stop-the-world pauses across actors
- All runtime allocations migrated to GC-managed per-actor heaps (snow_gc_alloc_actor)
- Bounded memory validated: long-running actors reclaim memory across 50 message cycles

**Stats:**
- 44 files modified (26 Rust source files)
- 57,657 lines of Rust (+1,118 net from v1.1)
- 2 phases, 6 plans
- 1 day (2026-02-07 → 2026-02-08)
- 22 commits

**Git range:** `feat(16-01)` → `feat(17-04)`

**What's next:** TBD -- all known issues resolved. Potential directions include distributed actors, hot code reloading, macros, generational GC, and precise stack scanning.

---

## v1.1 Language Polish (Shipped: 2026-02-08)

**Delivered:** Fixed all five documented v1.0 limitations -- multi-clause functions, string pattern matching, pipe operator with closures, actor-per-connection HTTP, and generic map types -- making the language feel complete and polished.

**Phases completed:** 11-15 (10 plans total)

**Key accomplishments:**
- Multi-clause function definitions with guard clauses, exhaustiveness warnings, and cross-clause type unification
- Full closure syntax rewrite with bare params, do/end body, multi-clause closures, and pipe-aware type checking
- Compile-time string pattern matching in case expressions via snow_string_eq
- Generic Map<K, V> types with string-key support, runtime key_type dispatch, and map literal syntax %{k => v}
- Actor-per-connection HTTP server with catch_unwind crash isolation replacing thread-per-connection model

**Stats:**
- 88 files modified
- 56,539 lines of Rust (+3,928 from v1.0)
- 5 phases, 10 plans
- 2 days (2026-02-07 → 2026-02-08)
- 45 commits

**Git range:** `feat(11-01)` → `feat(15-01)`

**What's next:** TBD -- all v1.0 limitations resolved. Potential directions include distributed actors, hot code reloading, macros, and mark-sweep GC.

---

## v1.0 MVP (Shipped: 2026-02-07)

**Delivered:** A statically typed, LLVM-compiled programming language with Elixir-style syntax, BEAM-style actor concurrency, supervision trees, a standard library for web backends, and full developer tooling.

**Phases completed:** 1-10 (55 plans total)

**Key accomplishments:**
- Full compiler pipeline (lexer, parser, HM type inference, MIR, LLVM codegen) producing native single-binary executables
- M:N work-stealing actor runtime with typed Pid<M>, 100K actor benchmark, process linking and exit signals
- OTP-style supervision trees with one_for_one/one_for_all/rest_for_one strategies and compile-time child spec validation
- Algebraic data types with Maranget's exhaustiveness/redundancy checking and ariadne diagnostics
- Standard library: I/O, strings, collections, file I/O, HTTP client/server, JSON encoding/decoding
- High-level concurrency: Service (GenServer) and Job (Task) abstractions with full type inference
- Developer tooling: code formatter, REPL with JIT, package manager, LSP server, VS Code extension

**Stats:**
- 107 Rust source files
- 52,611 lines of Rust
- 10 phases, 55 plans
- 2 days from start to ship (2026-02-05 → 2026-02-07)
- 213 commits

**Git range:** `feat(01-01)` → `feat(10-10)`

**What's next:** TBD -- language is feature-complete for v1. Potential v2 directions include distributed actors, hot code reloading, and macros.

---

## v8.0 Developer Tooling (Shipped: 2026-02-14)

**Delivered:** Made Mesh installable, editable, and navigable with one-command install scripts, complete TextMate grammar and Shiki themes, LSP code completion/signature help/formatting/document symbols, VS Code Marketplace publishing, REPL JIT audit for v7.0 symbols, and documentation corrections.

**Phases completed:** 81-86 (11 plans total)

**Key accomplishments:**
- Complete TextMate grammar covering all Mesh syntax (keywords, operators, doc comments, triple-quoted strings, module-qualified calls) with Shiki themes and hierarchical LSP document symbols
- GitHub Actions 6-target CI pipeline with LLVM static linking plus POSIX/PowerShell install scripts with platform detection, SHA-256 verification, and PATH configuration
- Four-tier LSP completion engine (scope-aware names, built-in types, keywords, snippets) and signature help with parameter tracking from type system
- VS Code extension published to Marketplace and Open VSX with icon, README, CHANGELOG (18KB VSIX)
- LSP textDocument/formatting wired to mesh-fmt with handlers for map/list literals and associated type bindings
- 80+ REPL JIT runtime symbol registrations for v7.0 iterators, collect, hash, timer, and stdlib gaps
- Documentation corrections: meshc binary name, curl installer URL, dynamic version badge, LLVM compilation description, project-based code examples

**Stats:**
- 80 files changed, +14,864 / -1,938 lines
- 98,836 lines of Rust (+1,646 from v7.0)
- 6 phases, 11 plans
- 1 day (2026-02-14)
- 59 commits
- 42/42 requirements satisfied

**Git range:** `feat(81-01)` -> `docs(phase-86)`

**What's next:** All 18 milestones complete. Potential directions include dot-triggered completion, semantic tokens, workspace symbols, Tree-sitter grammar, Homebrew packaging, inlay hints, or new language features.

---


## v11.0 Query Builder (Shipped: 2026-02-25)

**Delivered:** Expanded the Mesh ORM with comprehensive query builder capabilities (JOINs, aggregations, upserts, advanced WHERE, raw SQL fragments, RETURNING, subqueries) and rewrote all 68+ Mesher raw SQL data queries to use the ORM, verifying every addition end-to-end with runtime tests against SQLite and a live E2E smoke test of all Mesher HTTP/WS endpoints.

**Phases completed:** 11 phases (106-115 including 109.1), 22 plans

**Key accomplishments:**
- Advanced WHERE operators (NOT IN, BETWEEN, ILIKE, OR grouped conditions) and raw SQL fragments with $N renumbering across WHERE/SELECT/ORDER BY/GROUP BY positions
- JOIN support (inner, left, aliased, multi-table) and aggregation functions (count/sum/avg/min/max) with group_by and having — all runtime-verified against SQLite
- Upsert (INSERT ON CONFLICT DO UPDATE), RETURNING clause, and subquery WHERE IN; type checker arity and service loop type-dispatch bugs fixed (Phase 109.1)
- 49+ Mesher raw SQL queries rewritten to ORM across all domains (auth/sessions, issues/events, search, dashboard/analytics, alerts, retention/storage); 18 intentional ORM boundaries documented
- Mesher compiled zero errors and verified E2E — all 8 HTTP API domains return 2xx, WebSocket 101 confirmed, EventProcessor SIGSEGV resolved
- All 32 v11.0 requirements tracked and accepted; ROADMAP canonicalized with positional API acceptance, dead code removed

**Stats:**
- 76 files changed, +11,390 / -340 lines
- ~7,700 lines of Mesh (.mpl) | ~168,500 lines of Rust
- 11 phases, 22 plans
- 8 days (2026-02-17 → 2026-02-25)
- 82 commits
- 32/32 requirements satisfied

**Git range:** `feat(106-01)` → `docs(phase-115)`

---

