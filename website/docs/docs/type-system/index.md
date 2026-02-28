---
title: Type System
---

# Type System

Mesh has a powerful static type system with full type inference. You rarely need to write type annotations -- the compiler figures out types from context. When you do annotate, you get compile-time safety guarantees.

## Type Inference

The Mesh compiler infers types from how values are used. You can declare variables without annotations and the compiler determines the correct type:

```mesh
fn main() do
  let x = 42           # inferred as Int
  let name = "hello"   # inferred as String
  let pi = 3.14        # inferred as Float
  let active = true    # inferred as Bool
  println("${x} ${name} ${pi} ${active}")
end
```

Function return types can also be inferred:

```mesh
fn double(x :: Int) do
  x * 2
end
```

The compiler infers that `double` returns `Int` because `x * 2` produces an `Int`. You can always add explicit annotations for clarity:

```mesh
fn double(x :: Int) -> Int do
  x * 2
end
```

### When to Annotate

Type annotations are optional in many places, but recommended for:

- **Function signatures** -- makes the API clear to readers
- **Complex generic functions** -- helps the compiler and your teammates
- **Public interfaces** -- documents the contract

## Generics

Generic functions and types let you write code that works with any type. Use angle brackets to declare type parameters:

```mesh
struct Box<T> do
  value :: T
end deriving(Display, Eq)

fn main() do
  let b1 = Box { value: 42 }
  let b2 = Box { value: 42 }
  let bs = Box { value: "hello" }
  println("${b1}")
  println("${bs}")
  println("${b1 == b2}")
end
```

Generic functions use the same angle bracket syntax:

```mesh
fn apply(f :: Fun(Int) -> String, x :: Int) -> String do
  f(x)
end

fn apply2(f :: Fun(Int, Int) -> Int, a :: Int, b :: Int) -> Int do
  f(a, b)
end

fn main() do
  let result = apply(fn x -> "${x}" end, 42)
  println(result)

  let sum = apply2(fn a, b -> a + b end, 10, 20)
  println("${sum}")
end
```

Function types are written as `Fun(ParamTypes) -> ReturnType`. A zero-argument function type is `Fun() -> ReturnType`.

## Type Aliases

Type aliases give descriptive names to existing types without creating a new type. The `type Alias = ExistingType` syntax declares an alias that is completely transparent -- the compiler treats the alias and the aliased type as identical, so no conversion is ever needed.

```mesh
# Type aliases give descriptive names to existing types
type Url = String
type Count = Int

fn fetch_page(url :: Url) -> Int do
  String.length(url)   # Url is String -- no conversion needed
end

fn main() do
  let u :: Url = "https://example.com"
  println("${fetch_page(u)}")
end
```

### Pub Aliases for Cross-Module Use

Mark an alias `pub type` to export it so other modules can import it by name with `from Module import AliasName`. This lets you define a canonical name in one place and share it across the codebase without repeating the definition:

```mesh
# types/ids.mpl
pub type UserId = Int
pub type Email = String

# main.mpl
from Types.Ids import UserId, Email

fn create_profile(id :: UserId, email :: Email) -> String do
  "user-#{id}: #{email}"
end

fn main() do
  let result = create_profile(42, "alice@example.com")
  println(result)
end
```

### When to Use Type Aliases

Type aliases are useful when you want to:

- Add semantic meaning to primitive types (e.g., `UserId` vs `Int`, `Fingerprint` vs `String`)
- Document the intended use of a parameter without creating a new distinct type
- Share a type name across modules without repeating the underlying type definition

Type aliases in v13.0 are non-generic -- `type Pair<T> = {T, T}` is not yet supported. Use structs for parameterized types.

## Structs

Structs are product types -- they group multiple fields together. Define them with the `struct` keyword:

```mesh
struct Point do
  x :: Int
  y :: Int
end deriving(Eq, Ord, Display, Debug, Hash)
```

Create instances with curly brace syntax and access fields with dot notation:

```mesh
fn main() do
  let p = Point { x: 1, y: 2 }
  let q = Point { x: 1, y: 2 }
  let r = Point { x: 3, y: 4 }
  println("${p}")
  println("${p == q}")
  println("${p == r}")
end
```

Structs can be generic:

```mesh
struct Box<T> do
  value :: T
end deriving(Display, Eq)

fn main() do
  let b = Box { value: 42 }
  println("${b}")
end
```

## Sum Types

Sum types (also called algebraic data types or tagged unions) define a type that can be one of several variants. Use the `type` keyword:

```mesh
type Color do
  Red
  Green
  Blue
end
```

Variants are used directly by name. Pattern match on them with `case`:

```mesh
fn describe(c :: Color) -> Int do
  case c do
    Red -> 1
    Green -> 2
    Blue -> 3
  end
end

fn main() do
  let r = Red
  println("${describe(r)}")
  println("${describe(Green)}")
  println("${describe(Blue)}")
end
```

### Variants with Data

Variants can carry data. Mesh has built-in `Option` and `Result` types that follow this pattern:

```mesh
fn find_positive(a :: Int, b :: Int) -> Int? do
  if a > 0 do
    return Some(a)
  end
  if b > 0 do
    return Some(b)
  end
  None
end

fn main() do
  let r = find_positive(5, 10)
  case r do
    Some(val) -> println("${val}")
    None -> println("none")
  end
end
```

The `Int?` syntax is shorthand for `Option<Int>`. For error handling, use `Result` with the `!` shorthand:

```mesh
fn safe_divide(a :: Int, b :: Int) -> Int!String do
  if b == 0 do
    return Err("division by zero")
  end
  Ok(a / b)
end

fn compute(x :: Int) -> Int!String do
  let result = safe_divide(x, 2)?
  Ok(result + 10)
end

fn main() do
  let r = compute(20)
  case r do
    Ok(val) -> println("${val}")
    Err(msg) -> println(msg)
  end
end
```

The `?` operator propagates errors early -- if the expression evaluates to `Err` or `None`, the function returns immediately with that error.

## Traits

Traits define shared behavior that types can implement. Define a trait with the `interface` keyword and implement it with `impl`:

```mesh
interface Greeter do
  fn greet(self) -> String
end

struct Person do
  name :: String
end

impl Greeter for Person do
  fn greet(self) -> String do
    "Hello, I'm ${self.name}"
  end
end

fn main() do
  let p = Person { name: "Alice" }
  println(p.greet())
end
```

### Built-in Traits

Mesh provides several built-in traits:

| Trait | Purpose |
|-------|---------|
| `Eq` | Equality comparison (`==`, `!=`) |
| `Ord` | Ordering comparison (`<`, `>`, `<=`, `>=`) |
| `Display` | String representation for `println` and `"${...}"` |
| `Debug` | Detailed debug output |
| `Hash` | Hashing for use in maps and sets |
| `Json` | JSON serialization and deserialization |

### Trait Bounds on Functions

Function type annotations can use the `Fun()` type to accept functions as arguments, enabling higher-order programming:

```mesh
fn apply(f :: Fun(Int) -> String, x :: Int) -> String do
  f(x)
end

fn run_thunk(thunk :: Fun() -> Int) -> Int do
  thunk()
end

fn main() do
  let result = apply(fn x -> "${x}" end, 42)
  println(result)
  let val = run_thunk(fn -> 99 end)
  println("${val}")
end
```

## Deriving

Instead of manually implementing traits, you can derive them automatically. Add `deriving(...)` at the end of a struct or sum type definition:

```mesh
struct Point do
  x :: Int
  y :: Int
end deriving(Eq, Ord, Display, Debug, Hash)

fn main() do
  let p = Point { x: 1, y: 2 }
  let q = Point { x: 1, y: 2 }
  println("${p}")
  println("${p == q}")
end
```

### Deriving on Sum Types

Sum types support deriving the same traits:

```mesh
type Color do
  Red
  Green
  Blue
end deriving(Eq, Ord, Display, Debug, Hash)

fn main() do
  let r = Red
  let g = Green
  println("${r}")
  println("${g}")
  println("${r == r}")
  println("${r == g}")
end
```

### Selective Deriving

You can derive only the traits you need:

```mesh
struct Tag do
  id :: Int
end deriving(Eq)

fn main() do
  let a = Tag { id: 1 }
  let b = Tag { id: 1 }
  println("${a == b}")
end
```

### Deriving on Generic Types

Generic types can also derive traits:

```mesh
struct Box<T> do
  value :: T
end deriving(Display, Eq)

fn main() do
  let b1 = Box { value: 42 }
  let b2 = Box { value: 42 }
  let b3 = Box { value: 99 }
  println("${b1}")
  println("${b1 == b2}")
  println("${b1 == b3}")
end
```

### Available Derives

| Derive | What it generates |
|--------|-------------------|
| `Eq` | Structural equality comparison |
| `Ord` | Structural ordering (field-by-field or variant order) |
| `Display` | Human-readable string representation |
| `Debug` | Detailed debug string representation |
| `Hash` | Hash value computation |
| `Json` | JSON serialization and deserialization |

## Associated Types

Interfaces can declare associated types -- type members that implementing types must define. This enables generic protocols where the concrete types are determined by the implementation:

```mesh
interface Container do
  type Item
  fn first(self) -> Self.Item
end

struct IntBox do
  value :: Int
end

impl Container for IntBox do
  type Item = Int
  fn first(self) -> Int do
    self.value
  end
end

fn main() do
  let b = IntBox { value: 42 }
  println("${b.first()}")
end
```

Use `Self.Item` in method signatures to reference the associated type. The compiler resolves it to the concrete type from each implementation.

Interfaces can have multiple associated types:

```mesh
interface Mapper do
  type Input
  type Output
  fn apply(self) -> Self.Output
end
```

## Numeric Traits

Mesh provides built-in traits for arithmetic operators. Implement them to use `+`, `-`, `*`, `/` with your custom types:

| Trait | Operator | Method |
|-------|----------|--------|
| `Add` | `+` | `add(self, other)` |
| `Sub` | `-` | `sub(self, other)` |
| `Mul` | `*` | `mul(self, other)` |
| `Div` | `/` | `div(self, other)` |
| `Neg` | `-` (unary) | `neg(self)` |

Each numeric trait has an associated `type Output` that determines the result type:

```mesh
struct Vec2 do
  x :: Float
  y :: Float
end

impl Add for Vec2 do
  type Output = Vec2
  fn add(self, other :: Vec2) -> Vec2 do
    Vec2 { x: self.x + other.x, y: self.y + other.y }
  end
end

impl Neg for Vec2 do
  type Output = Vec2
  fn neg(self) -> Vec2 do
    Vec2 { x: 0.0 - self.x, y: 0.0 - self.y }
  end
end

fn main() do
  let a = Vec2 { x: 1.0, y: 2.0 }
  let b = Vec2 { x: 3.0, y: 4.0 }
  let sum = a + b
  let neg = -a
  println("${sum.x}, ${sum.y}")
  println("${neg.x}, ${neg.y}")
end
```

## From/Into Conversion

The `From` trait defines how to convert one type into another. Implement `From<SourceType> for TargetType` with a `from` function:

```mesh
struct Wrapper do
  value :: Int
end

impl From<Int> for Wrapper do
  fn from(n :: Int) -> Wrapper do
    Wrapper { value: n * 2 }
  end
end

fn main() do
  let w = Wrapper.from(21)
  println("${w.value}")
end
```

### Built-in Conversions

Mesh provides built-in `From` implementations for common type conversions:

| Conversion | Example | Result |
|------------|---------|--------|
| Int to Float | `Float.from(42)` | `42.0` |
| Int to String | `String.from(42)` | `"42"` |
| Float to String | `String.from(3.14)` | `"3.14"` |
| Bool to String | `String.from(true)` | `"true"` |

### Error Type Conversion with ?

When you implement `From<SourceError> for TargetError`, the `?` operator automatically converts error types. This lets you compose functions with different error types:

```mesh
struct AppError do
  message :: String
end

impl From<String> for AppError do
  fn from(msg :: String) -> AppError do
    AppError { message: msg }
  end
end

fn risky() -> Int!String do
  Err("something failed")
end

fn process() -> Int!AppError do
  let n = risky()?    # auto-converts String error to AppError
  Ok(n + 1)
end

fn main() do
  let r = process()
  case r do
    Ok(val) -> println("${val}")
    Err(e) -> println(e.message)
  end
end
```

## TryFrom/TryInto Conversion

`TryFrom` and `TryInto` are for fallible conversions -- conversions that can fail and return a `Result`. Where `From` always succeeds, `TryFrom` returns `Result<TargetType, ErrorType>` so callers can handle the failure case explicitly.

### Implementing TryFrom

Implement `TryFrom<SourceType>` for your type with a `try_from` function that returns `Result<Self, E>`. Call it via `TargetType.try_from(value)`:

```mesh
struct PositiveInt do
  value :: Int
end

impl TryFrom<Int> for PositiveInt do
  fn try_from(n :: Int) -> Result<PositiveInt, String> do
    if n > 0 do
      Ok(PositiveInt { value: n })
    else
      Err("must be positive")
    end
  end
end

fn main() do
  let r = PositiveInt.try_from(42)
  case r do
    Ok(p) -> println("${p.value}")    # prints: 42
    Err(e) -> println("error: ${e}")
  end
  let r2 = PositiveInt.try_from(-1)
  case r2 do
    Ok(p) -> println("${p.value}")
    Err(e) -> println("${e}")         # prints: must be positive
  end
end
```

### Automatic TryInto

When you implement `TryFrom<F>` for a type, `TryInto` is automatically available on the source type -- you never need to write a `TryInto` impl yourself. Call `.try_into()` on the source value with a type annotation so the compiler knows what target type to use:

```mesh
# No TryInto impl needed -- derived automatically from TryFrom<Int> for PositiveInt
fn main() do
  let r :: Result<PositiveInt, String> = 42.try_into()
  case r do
    Ok(p) -> println("${p.value}")    # prints: 42
    Err(e) -> println("error: ${e}")
  end
  let r2 :: Result<PositiveInt, String> = (-5).try_into()
  case r2 do
    Ok(p) -> println("${p.value}")
    Err(e) -> println("${e}")         # prints: must be positive
  end
end
```

### Using ? with TryFrom

The `?` operator works naturally with `try_from` and `try_into` results, just like it does with any `Result`. If the conversion fails, `?` propagates the `Err` immediately -- no manual case matching needed at the call site:

```mesh
fn double_positive(n :: Int) -> Int!String do
  let p = PositiveInt.try_from(n)?   # propagates Err if n <= 0
  Ok(p.value * 2)
end

fn main() do
  case double_positive(21) do
    Ok(v) -> println("${v}")         # prints: 42
    Err(e) -> println("error: ${e}")
  end
  case double_positive(-1) do
    Ok(v) -> println("${v}")
    Err(e) -> println("${e}")        # prints: must be positive
  end
end
```

TryFrom/TryInto is for fallible conversions. For infallible conversions, use [From/Into](#from-into-conversion).

## Next Steps

- [Iterators](/docs/iterators/) -- lazy iterator pipelines, combinators, and collection materialization
- [Concurrency](/docs/concurrency/) -- actors, message passing, and supervision
- [Syntax Cheatsheet](/docs/cheatsheet/) -- quick reference for all Mesh syntax
