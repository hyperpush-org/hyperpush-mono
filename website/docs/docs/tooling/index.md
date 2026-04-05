---
title: Developer Tools
description: Formatter, REPL, package manager, LSP, and editor support for Mesh
---

# Developer Tools

Mesh ships a developer toolchain centered on the `meshc` compiler plus the companion `meshpkg` package CLI. The verified public install path uses the documentation-served installer pair `https://meshlang.dev/install.sh` and `https://meshlang.dev/install.ps1` to place both binaries on your PATH before you choose a starter, configure formatting or testing, or wire Mesh into your editor.

> **Production backend proof:** This page stays focused on the public day-one CLI workflow first. When you later need the deeper maintainer-facing backend proof path, start with [Production Backend Proof](/docs/production-backend-proof/). That page is the handoff to the named backend proof surfaces instead of a first-contact repo-root runbook.

## Install the CLI tools

The staged release proof covers that installer pair for both `meshc` and `meshpkg` on these targets:

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

Verify the installed binaries before using the tooling below:

```bash
meshc --version
meshpkg --version
```

### Update an installed toolchain

If you installed Mesh through the public installers, refresh both binaries in place with either command:

```bash
meshc update
meshpkg update
```

Both commands rerun the canonical installer path and refresh both `meshc` and `meshpkg` together.

For the named maintainer-facing backend proof behind this public install contract, see [Production Backend Proof](/docs/production-backend-proof/).

If you are contributing to Mesh or need an unsupported target, build from source instead; treat that as an alternative workflow, not the primary public install contract.

## Package Manager

Mesh includes a built-in package manager for creating and managing projects.

Keep the public CLI workflow explicit and examples-first: hello world first, then the clustered scaffold, then the honest local SQLite starter or the serious shared/deployable PostgreSQL starter, and only after that the maintainer-facing backend proof page.

### Creating a New Project

Use `meshc init` to scaffold a new project:

```bash
meshc init my_app
```

This creates the following structure:

```
my_app/
  mesh.toml
  main.mpl
```

The generated `main.mpl` contains a minimal hello-world program:

```mesh
fn main() do
  IO.puts("Hello from Mesh!")
end
```

Use `meshc init --clustered` when you want the public clustered-app scaffold instead of the hello-world starter:

```bash
meshc init --clustered my_clustered_app
```

That scaffold adds:

- a package-only `mesh.toml`
- an `@cluster pub fn add()` boundary in `work.mpl`
- the generic `MESH_CLUSTER_COOKIE`, `MESH_NODE_NAME`, `MESH_DISCOVERY_SEED`, `MESH_CLUSTER_PORT`, `MESH_CONTINUITY_ROLE`, and `MESH_CONTINUITY_PROMOTION_EPOCH` contract in the generated README
- built-in operator guidance that points at the runtime-owned CLI instead of app-authored control-plane surfaces
- follow-on guidance that points at [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) for the serious shared/deployable starter and [`examples/todo-sqlite/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) for the honest local starter instead of internal proof fixtures

If you are migrating older clustered code, move `clustered(work)` into source-first `@cluster`, delete any `[cluster]` manifest stanza, and rename helper-shaped entries such as `execute_declared_work(...)` / `Work.execute_declared_work` to ordinary verbs like `add()` or `sync_todos()`. Keep the route-free `@cluster` surfaces canonical: the PostgreSQL Todo starter only dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`, while `GET /health` and mutating routes stay local. Default-count and two-node clustered-route behavior stay on the Distributed Proof handoff instead of the public starter contract.

If you want the honest local Todo starter, generate SQLite explicitly:

```bash
meshc init --template todo-api --db sqlite my_local_todo
```

The SQLite Todo starter is the honest local starter: a single-node SQLite Todo API with generated package tests, local `/health`, actor-backed write rate limiting, and Docker packaging around `meshc build .`. It does not claim `work.mpl`, `HTTP.clustered(...)`, `meshc cluster`, or clustered/operator proof surfaces.

When you need the serious shared or deployable Todo starter, generate Postgres instead:

```bash
meshc init --template todo-api --db postgres my_shared_todo
```

The PostgreSQL Todo starter keeps the clustered-function contract source-first and route-free: `work.mpl` stays on `@cluster pub fn sync_todos()`, `main.mpl` boots through `Node.start_from_env()`, `GET /todos` and `GET /todos/:id` dogfood explicit-count `HTTP.clustered(1, ...)`, `GET /health` plus mutating routes stay local, and the Dockerfile packages the binary produced by `meshc build .`. Treat the PostgreSQL starter as the fuller starter layered above the same route-free clustered contract, not as a replacement for the canonical route-free public surfaces. Keep the SQLite starter on its honest single-node contract instead of treating it as a clustered/operator proof surface.

Inspect a running clustered app with the same operator order used by the scaffold and [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md):

```bash
meshc cluster status <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> <request_key> --json
meshc cluster diagnostics <node-name@host:port> --json
```

Use the list form first to discover startup or request keys, then inspect a single continuity record. After that CLI order, keep the public follow-on ladder explicit:

- [Clustered Example](/docs/getting-started/clustered-example/) — the scaffold-first clustered app story
- [SQLite Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) — the honest local single-node starter
- [PostgreSQL Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) — the serious shared/deployable starter
- [Production Backend Proof](/docs/production-backend-proof/) — the maintainer-facing backend proof page after the starter/examples-first ladder

Keep the starter split explicit here too: [`examples/todo-sqlite/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) is the honest local starter with no `work.mpl`, `HTTP.clustered(...)`, or `meshc cluster` story, while [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) is the shared/deployable route-free starter that only dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`.

### Project Manifest

Every Mesh project has a `mesh.toml` file that describes the package and its dependencies:

```toml
[package]
name = "my_app"
version = "0.1.0"

[dependencies]
```

`main.mpl` stays the default executable entrypoint. When you need a different startup file, add the optional project-root-relative `[package].entrypoint = "lib/start.mpl"` override:

```toml
[package]
name = "my_app"
version = "0.1.0"
entrypoint = "lib/start.mpl"

[dependencies]
```

The manifest supports both **git** and **path** dependencies:

```toml
[dependencies]
my_lib = { path = "../my_lib" }
some_pkg = { git = "https://github.com/user/some_pkg", tag = "v1.0.0" }
```

Git dependencies support `rev`, `branch`, and `tag` specifiers for pinning to a specific version.

### Lockfile

When dependencies are resolved, a lockfile (`mesh.lock`) is generated to ensure reproducible builds. The lockfile records the exact version and source of every dependency in the project.

## Test Runner

Run all `*.test.mpl` files from a project root, a tests directory, or a specific test file with `meshc test`:

```bash
meshc test .
meshc test tests
meshc test tests/example.test.mpl
```

The test runner discovers all files ending in `.test.mpl` under the requested target, compiles and executes each independently, and prints a per-test pass/fail summary:

```
test arithmetic is correct ... ok
test string operations/length ... FAIL
  assert_eq failed: expected 5, got 6

2 tests, 1 failure
```

Exit code is non-zero if any test fails, making `meshc test` suitable for CI pipelines.

Coverage requests are intentionally honest today:

```bash
meshc test --coverage .
```

`--coverage` currently exits non-zero with an explicit unsupported message instead of claiming a stub report.

See the [Testing guide](/docs/testing/) for the full assertion API, grouping, mock actors, and receive expectations.

## Formatter

The Mesh formatter canonically formats your source code, enforcing a consistent style across your project:

```bash
meshc fmt main.mpl
```

To format a project directory:

```bash
meshc fmt .
```

To fail fast in CI or before committing if any file would change:

```bash
meshc fmt --check .
```

The formatter uses the **Wadler-Lindig** pretty-printing algorithm with a CST-based approach. This means:

- **Comments are preserved** -- the formatter works on the concrete syntax tree, so comments stay exactly where you put them
- **Whitespace and indentation are rewritten** canonically according to Mesh style conventions
- **Formatting is idempotent** -- running the formatter twice produces the same output as running it once

### Example

Before formatting:

```mesh
fn add(a,b) do
a+b
end
```

After `meshc fmt`:

```mesh
fn add(a, b) do
  a + b
end
```

### Format on Save

Mesh only publishes repo-owned format-on-save guidance for the first-class editors in the [support tiers](#support-tiers) below. In VS Code, the Mesh extension routes document formatting through `meshc lsp`. In Neovim, the repo-owned pack attaches the native `meshc lsp` client, so save-time formatting should use your normal Neovim LSP formatting hook. Best-effort editors should invoke `meshc fmt <file>` directly and treat that integration as user-maintained.

## REPL

The Mesh REPL (Read-Eval-Print Loop) provides interactive exploration with full language support:

```bash
meshc repl
```

This starts an interactive session where you can evaluate expressions, define functions, and explore the language:

```
mesh> 1 + 2
3 :: Int

mesh> let name = "Mesh"
"Mesh" :: String

mesh> fn double(x) do
  ...   x * 2
  ... end
Defined: double :: (Int) -> Int

mesh> double(21)
42 :: Int
```

The REPL uses **LLVM JIT compilation** under the hood, running the full compiler pipeline (parse, typecheck, MIR, LLVM IR) for every expression. This means REPL behavior is identical to compiled code -- there are no interpreter-specific quirks.

### REPL Commands

| Command | Shorthand | Description |
|---------|-----------|-------------|
| `:help` | `:h` | Show available commands |
| `:type <expr>` | `:t` | Show the inferred type without evaluating |
| `:quit` | `:q` | Exit the REPL |
| `:clear` | | Clear the screen |
| `:reset` | | Reset session (clear all definitions and history) |
| `:load <file>` | | Load and evaluate a Mesh source file |

### Multi-line Input

The REPL automatically detects incomplete input. If you open a `do` block without closing it with `end`, the REPL switches to continuation mode (shown by `...`) until all blocks are balanced:

```
mesh> fn greet(name) do
  ...   println("Hello, ${name}!")
  ... end
Defined: greet :: (String) -> Unit

mesh> greet("world")
Hello, world!
```

## meshpkg — Package Registry CLI

The `meshpkg` binary provides commands for publishing and consuming packages from the Mesh package registry.

### Authentication

Log in to the registry to store an API token locally:

```bash
meshpkg login
```

Credentials are stored in `~/.mesh/credentials`.

### Publishing a Package

Publish the current directory as a package:

```bash
meshpkg publish
```

This reads `mesh.toml`, creates a `.tar.gz` tarball, computes the SHA-256 checksum, and uploads to the registry. Publishing the same name+version twice is rejected (HTTP 409).

The publish archive preserves project-root-relative `.mpl` paths, including nested sources like `features/workflows/renderer.mpl` and override entries like `lib/start.mpl`, while still keeping `main.mpl` when it exists. Only visible source files are archived: hidden paths and test-only files such as `*.test.mpl` are excluded from the tarball.

### Installing a Package

Install the latest release of a package from the registry into the current project:

```bash
meshpkg install your-login/your-package
```

This fetches the latest published release, verifies its SHA-256 checksum, extracts it into the project's dependency directory, and updates mesh.lock to pin the exact version. Named install does not edit mesh.toml; add the dependency yourself when you want it declared in the manifest.

### Searching

Search the registry by name or keyword:

```bash
meshpkg search json
```

Returns matching package names and descriptions.

### mesh.toml with Registry Dependencies

Declare registry dependencies in `mesh.toml`:

```toml
[package]
name = "my_app"
version = "1.0.0"
description = "A Mesh application"
license = "MIT"

[dependencies]
"your-login/your-package" = "1.0.0"                         # registry: exact version (quoted because scoped names contain '/')
my_lib = { path = "../my_lib" }                              # local path
utils = { git = "https://github.com/user/utils", tag = "v1.0.0" }  # git
```

Scoped registry package names include `/`, so TOML keys must be quoted in `mesh.toml`.

Browse and search available packages at [packages.meshlang.dev](https://packages.meshlang.dev).

## Language Server (LSP)

Mesh includes a Language Server Protocol implementation that provides real-time feedback in your editor:

```bash
meshc lsp
```

This starts the language server on **stdin/stdout** using the **JSON-RPC** protocol (standard LSP transport). The server is built on the `tower-lsp` framework and provides:

### Features

The transport-level regression suite for `meshc lsp` now exercises these editor-facing behaviors against a small backend-shaped Mesh project over real stdio JSON-RPC:

| Feature | Description |
|---------|-------------|
| **Diagnostics** | Parse errors and type errors displayed inline as you type |
| **Hover** | Hover over identifiers to see inferred type information |
| **Go-to-definition** | Jump to definitions within backend-shaped project code |
| **Document formatting** | Format the current document through the same formatter used by `meshc fmt` |
| **Signature help** | Parameter hints for function calls, including active-parameter tracking |

The language server runs the full Mesh compiler pipeline (lexer, parser, type checker) on every keystroke, so diagnostics are always accurate and up to date.

### Configuration

The JSON-RPC transport is shared across editors, but Mesh only publishes repo-owned editor-host guidance for VS Code and Neovim. VS Code starts `meshc lsp` through the Mesh extension. Neovim uses the repo-owned pack in `tools/editors/neovim-mesh/`. Best-effort editors that support LSP can point their client at:

```json
{
  "command": "meshc",
  "args": ["lsp"]
}
```

## Editor Support

### Support tiers

| Tier | Editors | Mesh-owned contract |
|------|---------|---------------------|
| First-class | VS Code and Neovim | Public docs, editor-specific READMEs, and repo-owned proof cover the published install/run path. |
| Best-effort | Emacs, Helix, Zed, Sublime Text, TextMate reuse, and similar setups | Reuse the shared `meshc lsp` transport or VS Code TextMate grammar, but Mesh does not publish repo-owned editor-host smoke for these integrations. |

### VS Code

VS Code is a first-class editor host in the public Mesh tooling contract. The official Mesh extension provides syntax highlighting plus the `meshc lsp` features that now have transport-level proof on a small backend-shaped Mesh project: diagnostics, hover, go-to-definition, document formatting, and signature help. The current repo-owned proof stays intentionally bounded to same-file go-to-definition inside backend-shaped project code, clean diagnostics plus hover for a manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`, and shared grammar parity for `@cluster`, `@cluster(N)`, `#{...}`, and `${...}`. The extension is located in the `tools/editors/vscode-mesh/` directory of the Mesh repository.

#### Features

- **Syntax highlighting** via the shared TextMate grammar used by VS Code and the docs, with verified coverage for Mesh keywords, operators, comments, and both `#{...}` plus `${...}` interpolation in double- and triple-quoted strings
- **Language configuration** for bracket matching, auto-closing pairs, and automatic indentation of `do`/`end` blocks
- **Verified LSP integration** that starts `meshc lsp` automatically and exposes diagnostics, hover, go-to-definition, document formatting, and signature help

#### Installation

Install Mesh first with the same verified public installer pair above so `meshc lsp` is already on your PATH, then build the current packaged extension from source:

```bash
cd tools/editors/vscode-mesh
npm install
npm run compile
npm run package
```

The package step writes `dist/mesh-lang-<version>.vsix`. To install that freshly built artifact into your local VS Code profile, run:

```bash
npm run install-local
```

Or open the `tools/editors/vscode-mesh/` folder in VS Code and press F5 to launch an Extension Development Host with the extension loaded.

When you need the full repo-root public proof chain instead of only the VS Code packaging/install loop, run:

```bash
bash scripts/verify-m036-s03.sh
```

That verifier keeps the public tooling contract honest by replaying the docs contract, VitePress build, existing VSIX/public README proof, real VS Code editor-host smoke, and the Neovim replay from one named-phase command.

#### Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `mesh.lsp.path` | `"meshc"` | Path to the `meshc` binary (must be in PATH, or provide an absolute path) |

### Neovim

Neovim is a first-class editor host in the public Mesh tooling contract for the audited classic syntax plus native `meshc lsp` path already proven in `scripts/verify-m036-s02.sh`. The repo-owned support pack lives in `tools/editors/neovim-mesh/` and requires **Neovim 0.11+**.

#### Installation

Install Mesh first so `meshc` is available, then place `tools/editors/neovim-mesh/` on an active `packpath` as `pack/*/start/mesh-nvim`. A direct repo-local install looks like this:

```bash
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/nvim/site/pack/mesh/start"
ln -s \
  "/absolute/path/to/mesh-lang/tools/editors/neovim-mesh" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/nvim/site/pack/mesh/start/mesh-nvim"
```

After installation, opening any `*.mpl` file should load the classic syntax runtime files and auto-enable the native `meshc lsp` config when the binary is available.

#### Verification

For the full repo-root public tooling/editor proof chain, run:

```bash
bash scripts/verify-m036-s03.sh
```

Use the Neovim-specific verifier below when you only need to replay this pack's bounded proof surface:

```bash
NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh
```

That proof is intentionally bounded to the shared syntax corpus plus the native `meshc lsp` path. It does not imply Tree-sitter support or support for third-party Neovim plugin-manager packaging.

### Best-effort editors

Editors outside the first-class tier can still reuse the shared Mesh surfaces, but those integrations are best-effort. For syntax highlighting, reuse `tools/editors/vscode-mesh/syntaxes/mesh.tmLanguage.json` anywhere that can ingest a TextMate grammar. For LSP, point your editor at `meshc lsp` over stdin/stdout JSON-RPC.

Best-effort examples include Emacs, Helix, Zed, Sublime Text, and TextMate-style consumers of the shared grammar. Mesh does not publish repo-owned editor-host smoke, packaging, or troubleshooting guides for those setups.

## Assembled first-contact docs verifier

When you need the repo-root proof that the public first-contact docs still tell one coherent story — install Mesh, run hello-world, then deliberately choose the clustered scaffold, the honest local SQLite Todo starter, or the serious shared/deployable Postgres starter — run:

```bash
bash scripts/verify-m050-s02.sh
```

This verifier replays the slice-owned first-contact source contract, the retained M047 docs rails plus the retained M048 and M036 tooling contracts, then performs a serial `npm --prefix website run build` and copies built HTML snapshots for Getting Started, Clustered Example, and Tooling into `.tmp/m050-s02/verify/` for diagnosis.

## Release Assembly Runbook

When you need the full public-release acceptance flow instead of an individual tool check, run the assembled verifier from the repo root with the repo `.env` loaded:

```bash
set -a && source .env && set +a && bash scripts/verify-m034-s05.sh
```

The candidate identity stays split on purpose:

- Binary release candidate tag: `v<Cargo version>` from `compiler/meshc/Cargo.toml` and `compiler/meshpkg/Cargo.toml`
- VS Code extension release candidate tag: `ext-v<extension version>` from `tools/editors/vscode-mesh/package.json`

Hosted rollout evidence must exist for these exact workflows:

- `deploy.yml`
- `deploy-services.yml`
- `authoritative-verification.yml`
- `release.yml`
- `extension-release-proof.yml`
- `publish-extension.yml`

The runbook stays tied to these exact public URLs:

- `https://meshlang.dev/install.sh`
- `https://meshlang.dev/install.ps1`
- `https://meshlang.dev/docs/getting-started/`
- `https://meshlang.dev/docs/tooling/`
- `https://packages.meshlang.dev/packages/snowdamiz/mesh-registry-proof`
- `https://packages.meshlang.dev/search?q=snowdamiz%2Fmesh-registry-proof`
- `https://api.packages.meshlang.dev/api/v1/packages?search=snowdamiz%2Fmesh-registry-proof`

The verifier persists the candidate and hosted-run evidence under:

- `.tmp/m034-s05/verify/candidate-tags.json`
- `.tmp/m034-s05/verify/remote-runs.json`

## Assembled contract verifier

When you need the retained repo-root proof for installer-backed updates, optional override entrypoints, package publish/archive truth, shared grammar parity, and the bounded editor surface, run:

```bash
bash scripts/verify-m048-s05.sh
```

## Assembled scaffold/example verifier

When you need the repo-root proof that the public Todo onboarding story still stays scaffold/examples-first — SQLite remains the honest local starter, Postgres remains the serious shared/deployable path, and the retained M048/tooling guardrails stay green underneath that split — run:

```bash
bash scripts/verify-m049-s05.sh
```

This assembled verifier replays the new first-contact docs preflight, the dual-db scaffold rails, the direct `/examples` parity check, the retained clustered proof wrappers, and the retained M048 tooling verifier, then publishes one retained bundle under `.tmp/m049-s05/verify/` for diagnosis.

## Tool Summary

| Tool | Command | Description |
|------|---------|-------------|
| Formatter | `meshc fmt [path]` | Canonically format Mesh source code or use `--check` in CI |
| REPL | `meshc repl` | Interactive evaluation with LLVM JIT |
| Package Manager | `meshc init [name]` | Create a new Mesh project |
| Test Runner | `meshc test [path]` | Run `*.test.mpl` files from a project root, tests directory, or specific test file |
| Package CLI | `meshpkg <command>` | Publish, install, and search registry packages |
| Language Server | `meshc lsp` | JSON-RPC LSP server for diagnostics, hover, formatting, navigation, and signature help |
| VS Code Extension | -- | First-class VS Code editor host with verified Mesh LSP integration |
| Neovim Pack | -- | First-class Neovim editor host for the classic syntax plus native `meshc lsp` path |

## Next Steps

- [Testing](/docs/testing/) -- write and run tests with `meshc test`
- [Standard Library](/docs/stdlib/) -- Crypto, Encoding, and DateTime modules
- [Language Basics](/docs/language-basics/) -- core language features and syntax
- [Distributed Actors](/docs/distributed/) -- building distributed systems with Mesh
