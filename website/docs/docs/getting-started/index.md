---
title: Getting Started
description: Install Mesh, run hello-world, then choose the starter path that matches your next job.
---

# Getting Started

This guide takes you through the public first-contact path: install Mesh, run hello-world, then choose the starter that matches what you want to evaluate next.

## Installation

Use the documented installer scripts to install both `meshc` and `meshpkg`. The staged release proof covers these installer targets:

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

The installers place both binaries in `~/.mesh/bin` on Unix-like systems and `~\.mesh\bin` on Windows.

### Verify the install

After installing, verify both binaries are available:

```bash
meshc --version
meshpkg --version
```

You should see the Mesh version number printed for each command.

### Alternative: Build from source

If you are contributing to Mesh or targeting an environment outside the public installer coverage, build from source instead. Treat this as an alternative workflow, not the primary public install path:

```bash
git clone https://github.com/snowdamiz/mesh-lang.git
cd mesh-lang
cargo install --path compiler/meshc
cargo install --path compiler/meshpkg
```

## Hello World

Create a new Mesh project:

```bash
meshc init hello
cd hello
```

Open `main.mpl` and replace its contents with:

```mesh
fn main() do
  println("Hello, World!")
end
```

Compile and run it:

```bash
meshc build .
./hello
```

You should see `Hello, World!` printed to the terminal.

`main.mpl` remains the default executable entrypoint. If you need a different startup file later, use the optional `[package].entrypoint = "lib/start.mpl"` setting in `mesh.toml`.

## Choose your next starter

Once hello-world runs, pick the starter that matches your next job.

- `meshc init --clustered hello_cluster` — the minimal clustered starter. It keeps the public clustered-app contract small: `work.mpl` declares `@cluster`, `main.mpl` boots through `Node.start_from_env()`, and runtime inspection stays on Mesh-owned `meshc cluster status|continuity|diagnostics` commands.
- `meshc init --template todo-api --db sqlite todo_api` — the honest local starter. It is a single-node SQLite Todo API with actor-backed write rate limiting, generated package tests, and no clustered placement or operator claims.
- `meshc init --template todo-api --db postgres shared_todo` — the serious shared/deployable PostgreSQL starter. It keeps clustered work source-first, uses migrations plus a real `DATABASE_URL`, and dogfoods `HTTP.clustered(1, ...)` only on the shared read routes while local health and mutating routes stay local.

## What's Next?

Keep the public first-contact ladder explicit and ordered: clustered scaffold first, then the honest local SQLite starter, then the serious shared/deployable PostgreSQL starter, and only then the maintainer-facing backend proof page.

- [Clustered Example](/docs/getting-started/clustered-example/) -- the scaffold-first clustered tutorial using `meshc init --clustered`
- [SQLite Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) -- the honest local single-node Todo starter
- [PostgreSQL Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) -- the serious shared/deployable Todo starter
- [Production Backend Proof](/docs/production-backend-proof/) -- the maintainer-facing backend proof page after the starter/examples-first ladder

After that starter/examples-first ladder, continue with the language guides:

- [Language Basics](/docs/language-basics/) -- variables, types, functions, pattern matching, control flow, and more
- [Type System](/docs/type-system/) -- structs, sum types, generics, and type inference
- [Concurrency](/docs/concurrency/) -- actors, message passing, supervision, and services
