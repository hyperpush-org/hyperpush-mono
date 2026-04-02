<div align="center">

# Mesh Language

![Version](https://img.shields.io/badge/version-v12.0-blue.svg?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)
![Build](https://img.shields.io/badge/build-passing-success.svg?style=flat-square)

**Expressive, readable concurrency.**
*Elixir-style syntax. Static type inference. Native single binaries.*

[Documentation](https://meshlang.dev) • [Production Proof](https://meshlang.dev/docs/production-backend-proof/) • [Distributed Proof](https://meshlang.dev/docs/distributed-proof/) • [Get Started](#quick-start) • [Contributing](#contributing)

</div>

---

## What is Mesh?

Mesh is a general-purpose programming language designed to make concurrent software scalable, fault-tolerant, and maintainable. It combines the **expressive syntax of Ruby/Elixir** and the **fault-tolerant actor model of Erlang/BEAM** with **static Hindley-Milner type inference** and **native performance via LLVM**.

Mesh compiles directly to a standalone native binary—no virtual machine to install, no heavy runtime to manage.

## Key Features

### Safety Without Verbosity
- **Static Type System:** Full compile-time type checking with Hindley-Milner inference. You rarely need to write type annotations.
- **Null Safety:** No nulls. Use `Option<T>` and `Result<T, E>` with pattern matching.
- **Pattern Matching:** Exhaustive pattern matching on all types, ensuring you handle every case.

### Concurrency & Reliability
- **Actor Model:** Lightweight processes (green threads) isolated by default. Spawn 100k+ actors in seconds.
- **Fault Tolerance:** Supervision trees and "let it crash" philosophy. If an actor crashes, its supervisor restarts it—the rest of your app stays up.
- **Message Passing:** Actors communicate exclusively via immutable messages. No shared memory, no data races.
- **Distributed Mesh:** Seamlessly cluster nodes. Send messages to remote actors as easily as local ones using location-transparent PIDs.

### Backend-Capable Runtime
- **Native Binaries:** Compiles to a single, self-contained executable. Easy to deploy (copy-paste).
- **Batteries Included:**
  - Built-in **PostgreSQL** & **SQLite** drivers with connection pooling.
  - **WebSocket** server support (actor-per-connection).
  - **JSON** serialization/deserialization.
  - **HTTP** server with routing and middleware.
- **Modern Tooling:** Built-in project scaffolding (`meshc init`), formatter (`meshc fmt`), test runner (`meshc test <project-or-dir>`), and Language Server Protocol (LSP) support for your editor.
- **String ergonomics:** `#{}` string interpolation, multiline heredocs, regex literals, and `Env.get`/`Env.get_int` for environment variables.
- **Slot pipe operator:** Route piped values to any argument position with `|N>` syntax.

## Quick Start

### 1. Install Mesh

The verified public install path uses the documentation-served installer pair `https://meshlang.dev/install.sh` and `https://meshlang.dev/install.ps1` to install both `meshc` and `meshpkg`. The staged release proof covers these installer targets:

- macOS `x86_64` and `arm64`
- Linux `x86_64` and `arm64` (GNU libc)
- Windows `x86_64`

**macOS and Linux:**

```bash
curl -sSf https://meshlang.dev/install.sh | sh
```

**Windows x86_64 (PowerShell):**

```powershell
irm https://meshlang.dev/install.ps1 | iex
```

Verify the installed binaries:

```bash
meshc --version
meshpkg --version
```

Refresh an installer-backed toolchain in place with either binary:

```bash
meshc update
meshpkg update
```

Both commands rerun the canonical installer path and refresh both installed tools together.

**Alternative: build from source (contributors / unsupported targets; Rust + LLVM required):**

Source builds are still supported, but they are an explicit alternative workflow rather than the public install path proven by the release process.

```bash
git clone https://github.com/snowdamiz/mesh-lang.git
cd mesh-lang
cargo install --path compiler/meshc
cargo install --path compiler/meshpkg
```

When you need the assembled repo-root proof for override-entry projects, toolchain updates, grammar parity, and the refreshed public touchpoints, run:

```bash
bash scripts/verify-m048-s05.sh
```

### 2. Optional: Scaffold a Project

```bash
meshc init hello_mesh
cd hello_mesh
```

This creates a Mesh project directory with a `mesh.toml` manifest and `main.mpl`; `main.mpl` remains the default executable entrypoint.

When you need a different startup file, keep the override project-root-relative and set the optional `[package].entrypoint = "lib/start.mpl"` in `mesh.toml`:

```toml
[package]
name = "hello_mesh"
version = "0.1.0"
entrypoint = "lib/start.mpl"
```

If you want the public clustered-app scaffold instead of the hello-world app, generate it explicitly:

```bash
meshc init --clustered hello_cluster
cd hello_cluster
```

The clustered scaffold keeps `mesh.toml` package-only, declares `@cluster pub fn add()` in `work.mpl`, derives the runtime-owned handler name as `Work.add`, uses the generic `MESH_*` runtime contract, and points operators at the runtime-owned inspection commands:

```bash
meshc cluster status <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> <request_key> --json
meshc cluster diagnostics <node-name@host:port> --json
```

If you want a fuller starter that keeps the same source-first clustered contract while adding a SQLite HTTP app, generate the Todo template instead:

```bash
meshc init --template todo-api todo_api
cd todo_api
```

That template keeps `work.mpl` on the ordinary source-first declaration `@cluster pub fn sync_todos()`, adds several `HTTP.on_get`/`HTTP.on_post`/`HTTP.on_put`/`HTTP.on_delete` routes, dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`, keeps `GET /health` plus mutating routes local, wires in actor-backed rate limiting, and ships a Dockerfile that packages the binary produced by `meshc build .` instead of assuming access to the Mesh repo.

If you are migrating older clustered code, move `clustered(work)` into source-first `@cluster`, delete any `[cluster]` manifest stanza, and rename helper-shaped entries such as `execute_declared_work(...)` / `Work.execute_declared_work` to ordinary verbs like `add()` or `sync_todos()`. Keep the route-free `@cluster` surfaces canonical: the Todo starter only dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`, while `GET /health` and mutating routes stay local. Default-count and two-node clustered-route behavior stay on the repo S07 rail (`cargo test -p meshc --test e2e_m047_s07 -- --nocapture`).

The primary clustered-app story still starts with `meshc init --clustered`, but it now shares one canonical route-free contract with [`tiny-cluster/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/tiny-cluster/README.md) and [`cluster-proof/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/cluster-proof/README.md). The Todo template is the fuller starter layered on top of that same contract, not a replacement for the route-free public surfaces. Use the continuity list form first to discover startup or request keys, then inspect a single continuity record.

### 3. Hello World

Create a file named `hello.mpl`:

```elixir
actor greeter() do
  receive do
    msg -> println("Nice to meet you, #{msg}!")
  end
end

fn main() do
  println("Hello, Mesh world!")

  # Spawn an actor and send it a message
  let pid = spawn(greeter)
  send(pid, "Developer")
end
```

Run it:

```bash
meshc build hello.mpl
./hello
```

### 4. A Web Server Example

```elixir
struct User do
  id :: Int
  name :: String
  email :: String
end

fn home_handler(request) do
  HTTP.response(200, "Welcome to Mesh!")
end

fn main() do
  let r = HTTP.router()
  let r = HTTP.on_get(r, "/", home_handler)
  HTTP.serve(r, 8080)
end
```

## Production Backend Proof

The quick-start examples above are intentionally small. If you want the real backend proof surface instead of inferring readiness from tutorials, start here:

- [Production Backend Proof](https://meshlang.dev/docs/production-backend-proof/) — public map of the named build, deploy, supervision, and documentation-proof checks
- [`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md) — the deepest repo runbook with the authoritative backend commands and proof targets

## Distributed Proof

The public clustered-app story follows one runtime-owned inspection flow:

```bash
meshc cluster status <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> <request_key> --json
meshc cluster diagnostics <node-name@host:port> --json
```

New clustered packages should declare route-free startup work with `@cluster` in `work.mpl` while keeping `mesh.toml` package-only. If you are migrating older clustered code, move `clustered(work)` into source-first `@cluster`, delete any `[cluster]` manifest stanza, and rename helper-shaped entries such as `execute_declared_work(...)` / `Work.execute_declared_work` to ordinary verbs like `add()` or `sync_todos()`. Keep the route-free `@cluster` surfaces canonical: the Todo starter only dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`, while `GET /health` and mutating routes stay local. Default-count and two-node clustered-route behavior stay on the repo S07 rail (`cargo test -p meshc --test e2e_m047_s07 -- --nocapture`).

When you need the public clustered story, start with any of the three canonical route-free surfaces, then use the proof rails when you need verifier coverage behind those runtime-owned commands or the fuller Todo starter:

- [Clustered Example](https://meshlang.dev/docs/getting-started/clustered-example/) — public scaffold-first walkthrough for `meshc init --clustered`
- [`tiny-cluster/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/tiny-cluster/README.md) — the smallest repo-owned route-free package surface
- [`cluster-proof/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/cluster-proof/README.md) — the deeper packaged failover/operator runbook
- `meshc init --template todo-api <name>` — the fuller starter that keeps the same source-first `@cluster` contract while adding local SQLite/HTTP routes, explicit-count `HTTP.clustered(1, ...)` on the selected read routes, and Docker packaging
- [Distributed Proof](https://meshlang.dev/docs/distributed-proof/) — public map of the route-free canonical surfaces, the fuller Todo starter, bounded automatic promotion, runtime-owned authority fields, stale-primary fencing, and the read-only Fly evidence path
- `bash scripts/verify-m047-s04.sh` — the authoritative cutover rail for the source-first route-free clustered contract
- `bash scripts/verify-m047-s05.sh` — the lower-level Todo/runtime subrail that proves the fuller starter natively and inside Docker
- `cargo test -p meshc --test e2e_m047_s07 -- --nocapture` — the repo S07 rail for default-count and two-node `HTTP.clustered(...)` behavior beyond the Todo starter's explicit-count read routes
- `bash scripts/verify-m047-s06.sh` — the final closeout rail that wraps S05, rebuilds docs truth, and owns the assembled `.tmp/m047-s06/verify` bundle
- `bash scripts/verify-m046-s06.sh` — the historical M046 closeout wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m046-s05.sh` — the historical M046 equal-surface wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m046-s04.sh` — the historical M046 package/startup wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m045-s05.sh` — the historical M045 closeout wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m045-s04.sh` — the historical M045 assembled wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m045-s03.sh` — the historical failover-specific subrail

## Public Release Candidate Runbook

When you need the assembled public-release proof instead of subsystem-local checks, run the canonical S05 verifier from the repo root:

```bash
set -a && source .env && set +a && bash scripts/verify-m034-s05.sh
```

The release candidate identity is intentionally split instead of pretending Mesh ships on one unified tag:

- Binary candidate tag: `v<Cargo version>` derived from `compiler/meshc/Cargo.toml` and `compiler/meshpkg/Cargo.toml` (those Cargo versions must stay aligned)
- VS Code extension candidate tag: `ext-v<extension version>` derived from `tools/editors/vscode-mesh/package.json`

Hosted rollout evidence must exist for these exact workflows before the release is considered public-ready:

- `deploy.yml`
- `deploy-services.yml`
- `authoritative-verification.yml`
- `release.yml`
- `extension-release-proof.yml`
- `publish-extension.yml`

The assembled proof command also checks these exact public URLs:

- `https://meshlang.dev/install.sh`
- `https://meshlang.dev/install.ps1`
- `https://meshlang.dev/docs/getting-started/`
- `https://meshlang.dev/docs/tooling/`
- `https://packages.meshlang.dev/packages/snowdamiz/mesh-registry-proof`
- `https://packages.meshlang.dev/search?q=snowdamiz%2Fmesh-registry-proof`
- `https://api.packages.meshlang.dev/api/v1/packages?search=snowdamiz%2Fmesh-registry-proof`

After every run, inspect these proof artifacts before calling the candidate public-ready:

- `.tmp/m034-s05/verify/candidate-tags.json`
- `.tmp/m034-s05/verify/remote-runs.json`

## Performance

Measured on dedicated Fly.io `performance-2x` VMs (2 vCPU, 4 GB RAM), each server running alone (isolated), load generator in the same region over Fly.io's private WireGuard network. 100 concurrent connections, 30 s timed runs × 4 (run 1 excluded, runs 2–5 averaged).

| Language | /text req/s | /json req/s | /text p99 | /json p99 |
|----------|------------|------------|-----------|-----------|
| **Mesh** | **29,108** | **28,955** | 16.94 ms  | 16.19 ms  |
| Go       | 30,306     | 29,934     | 8.51 ms   | 8.40 ms   |
| Rust     | 46,244     | 46,234     | 4.55 ms   | 4.77 ms   |
| Elixir   | 12,441     | 12,733     | 25.14 ms  | 23.41 ms  |

[Full results and methodology →](benchmarks/RESULTS.md)

## Documentation

Full documentation, including guides and API references, is available at **[meshlang.dev](https://meshlang.dev)**.

For the canonical backend proof story, use **[Production Backend Proof](https://meshlang.dev/docs/production-backend-proof/)** and the repo runbook at [`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md).

For the canonical distributed clustered-work proof story, use **[Distributed Proof](https://meshlang.dev/docs/distributed-proof/)** plus the equal-surface runbooks at [`tiny-cluster/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/tiny-cluster/README.md) and [`cluster-proof/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/cluster-proof/README.md).

## Project Status

Mesh is currently in active development.

- **Current Stable:** v12.0 (Language Ergonomics & Open Source Readiness)
- **Recent additions:** Slot pipe operator (`|2>`), `#{}` string interpolation, heredocs, regex literals, environment variable stdlib, and performance benchmarks vs Go, Rust, and Elixir.

See [ROADMAP.md](.planning/ROADMAP.md) for detailed planning and architectural decisions.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
