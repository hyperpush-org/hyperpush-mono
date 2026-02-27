<div align="center">

# Mesh Language

![Version](https://img.shields.io/badge/version-v12.0-blue.svg?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)
![Build](https://img.shields.io/badge/build-passing-success.svg?style=flat-square)

**Expressive, readable concurrency.**
*Elixir-style syntax. Static type inference. Native single binaries.*

[Documentation](https://meshlang.dev) • [Get Started](#quick-start) • [Contributing](#contributing)

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

### Production Ready
- **Native Binaries:** Compiles to a single, self-contained executable. Easy to deploy (copy-paste).
- **Batteries Included:**
  - Built-in **PostgreSQL** & **SQLite** drivers with connection pooling.
  - **WebSocket** server support (actor-per-connection).
  - **JSON** serialization/deserialization.
  - **HTTP** server with routing and middleware.
- **Modern Tooling:** Built-in package manager, formatter (`mesh fmt`), and Language Server Protocol (LSP) support for your editor.
- **String ergonomics:** `#{}` string interpolation, multiline heredocs, regex literals, and `Env.get`/`Env.get_int` for environment variables.
- **Slot pipe operator:** Route piped values to any argument position with `|N>` syntax.

## Quick Start

### 1. Installation

**From Source (Rust required):**

```bash
git clone https://github.com/mesh-lang/mesh.git
cd mesh
cargo install --path compiler/meshc
```

### 2. Hello World

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

### 3. A Web Server Example

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

Full documentation, including guides and API references, is available at **[meshlang.dev](https://meshlang.dev)** (placeholder link).

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
