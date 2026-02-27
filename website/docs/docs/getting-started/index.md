---
title: Getting Started
---

# Getting Started

This guide will take you from zero to running your first Mesh program. By the end, you will have Mesh installed, understand the basic project structure, and have compiled and run a working program.

## What is Mesh?

Mesh is a statically-typed, compiled programming language designed for expressive, readable concurrency. It combines the actor model from Erlang/Elixir with a modern type system, pattern matching, and native compilation via LLVM.

Key properties of Mesh:

- **Statically typed with inference** -- the compiler catches type errors at compile time, but you rarely need to write type annotations thanks to type inference
- **Compiles to native code** -- Mesh compiles via LLVM to produce fast native binaries
- **Actor-based concurrency** -- lightweight actors with message passing, supervision trees, and fault tolerance built into the language
- **Familiar syntax** -- inspired by Elixir and Rust, with `do...end` blocks, pattern matching, and pipe operators

## Installation

Install Mesh with a single command:

```bash
curl -sSf https://mesh-lang.org/install.sh | sh
```

This downloads the latest `meshc` binary and adds it to your PATH.

### Verifying Installation

After installing, verify the compiler is available:

```bash
meshc --version
```

You should see the Mesh version number printed to the terminal.

## Hello World

Create a new Mesh project:

```bash
meshc init hello
cd hello
```

Open `main.mpl` -- meshc generates a starter file for you. Replace its contents with:

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

Let's break down what's happening:

- `fn main()` declares the entry point of the program -- every Mesh program starts here
- `do...end` defines the function body
- `println` prints a string to stdout followed by a newline
- Mesh source files use the `.mpl` file extension

## Your First Program

Now let's write something more interesting. Open `main.mpl` and replace its contents with:

```mesh
fn greet(name :: String) -> String do
  "Hello, #{name}!"
end

fn main() do
  let message = greet("Mesh")
  println(message)
end
```

Compile and run it:

```bash
meshc build .
./hello
```

This prints `Hello, Mesh!`. Here's what's new:

- `let` creates a variable binding -- variables in Mesh are immutable by default
- `::` provides a type annotation -- `name :: String` means the parameter `name` has type `String`
- `->` declares the return type -- `-> String` means the function returns a `String`
- `"#{name}"` is string interpolation -- expressions inside `#{}` are evaluated and inserted into the string. The older `${}` syntax also works: both are valid.
- The last expression in a function is its return value -- no explicit `return` keyword needed

### Adding More Functions

Let's extend the program with some arithmetic:

```mesh
fn add(a :: Int, b :: Int) -> Int do
  a + b
end

fn double(x :: Int) -> Int do
  x * 2
end

fn main() do
  let sum = add(10, 20)
  println("#{sum}")

  let result = double(7)
  println("#{result}")

  let greeting = "Mesh"
  println("Hello, #{greeting}!")
end
```

This demonstrates:

- Functions with multiple parameters
- `Int` type for integers
- String interpolation with expressions: `"#{sum}"` converts the integer to a string automatically

### Using the Pipe Operator

Mesh has a pipe operator `|>` that passes the result of one function as the first argument to the next:

```mesh
fn double(x :: Int) -> Int do
  x * 2
end

fn add_one(x :: Int) -> Int do
  x + 1
end

fn main() do
  let result = 5 |> double |> add_one
  println("#{result}")
end
```

This prints `11`. The expression `5 |> double |> add_one` is equivalent to `add_one(double(5))` -- it reads left to right, making chains of transformations easy to follow.

## What's Next?

Now that you have Mesh installed and running, explore the language in depth:

- [Language Basics](/docs/language-basics/) -- variables, types, functions, pattern matching, control flow, and more
- [Type System](/docs/type-system/) -- structs, sum types, generics, and type inference
- [Concurrency](/docs/concurrency/) -- actors, message passing, supervision, and services
