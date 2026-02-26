---
name: mesh-traits
description: Mesh trait system: interface definitions, impl blocks, deriving macros (Json, Row, Display, Eq, Ord), and associated types.
---

# Mesh Traits

See also: skills/types for struct definitions used in deriving examples.

## Interfaces (Trait Definitions)
1. Defined with `interface Name do method_signatures end`.
2. Methods can have a `self` parameter for instance methods.
3. Associated types declared with `type Item` inside the interface body.
4. Interfaces describe a contract — types implement the contract via `impl`.

```mesh
interface Container do
  type Item
  fn first(self) -> Self.Item
end
```

## Impl Blocks
1. `impl InterfaceName for TypeName do ... end` provides the implementation.
2. Associated types are assigned: `type Item = Int`.
3. Method bodies are full function implementations with `do...end`.
4. `self` refers to the current instance (bound by the struct/type being implemented).
5. Multiple interfaces can be implemented for the same type.

```mesh
struct IntPair do
  a :: Int
  b :: Int
end

struct StrPair do
  a :: String
  b :: String
end

impl Container for IntPair do
  type Item = Int
  fn first(self) -> Int do
    42
  end
end

impl Container for StrPair do
  type Item = String
  fn first(self) -> String do
    "hello"
  end
end

fn main() do
  let ip = IntPair { a: 1, b: 2 }
  let i = ip.first()   # calls Container impl
  println(i.to_string())
end
```

## Deriving Macros
1. `end deriving(Protocol1, Protocol2)` on a struct/type automatically generates implementations.
2. Multiple protocols can be derived in one `deriving(...)` clause.
3. `deriving` only applies to `struct` and `type` (ADT) definitions.
4. Only derive what you need — unused protocols add compile overhead.

### Json
1. `deriving(Json)` generates `Json.encode(val) -> String` and `TypeName.from_json(str) -> Result<TypeName, String>`.
2. Field names in JSON match the struct field names exactly.
3. Nested structs require the inner struct to also derive Json.

```mesh
struct User do
  name :: String
  age :: Int
  score :: Float
  active :: Bool
end deriving(Json)

let u = User { name: "Alice", age: 30, score: 95.5, active: true }
let json = Json.encode(u)
println(json)   # {"name":"Alice","age":30,"score":95.5,"active":true}

let result = User.from_json(json)
case result do
  Ok(u2) -> println("#{u2.name} #{u2.age}")
  Err(e) -> println("Error: #{e}")
end
```

### Row
1. `deriving(Row)` generates `TypeName.from_row(map) -> Result<TypeName, String>`.
2. The row is a `Map<String, String>` — all values are strings (as returned by database drivers).
3. Automatic type coercion: Int fields parse from string, Float from string, Bool from "t"/"true"/"1".
4. Used with `Sqlite.query` and `Pg.query` results.

```mesh
struct User do
  name :: String
  age :: Int
  score :: Float
  active :: Bool
end deriving(Row)

let row = Map.new()
let row = Map.put(row, "name", "Alice")
let row = Map.put(row, "age", "30")
let row = Map.put(row, "score", "95.5")
let row = Map.put(row, "active", "t")

case User.from_row(row) do
  Ok(u) -> println("#{u.name} #{u.age}")
  Err(e) -> println("Error: #{e}")
end
```

### Display
1. `deriving(Display)` generates `.to_string()` method and enables `"#{val}"` interpolation.
2. Structs render as `TypeName { field: value, ... }`.
3. Required for using a struct value inside string interpolation.

### Eq
1. `deriving(Eq)` generates `==` and `!=` operators.
2. Comparison is field-by-field structural equality.
3. All fields must also implement `Eq`.

### Ord
1. `deriving(Ord)` generates `<`, `>`, `<=`, `>=` operators.
2. Comparison is lexicographic: first field first, then second, etc.
3. Requires `Eq` to also be derived or implemented.

```mesh
struct Box<T> do
  value :: T
end deriving(Display, Eq)

let b1 = Box { value: 42 }
let b2 = Box { value: 42 }
let b3 = Box { value: 99 }
println("#{b1 == b2}")   # true
println("#{b1 == b3}")   # false
```

### Sum Type Deriving
1. `type` (ADT) can also derive Json.
2. Each variant serializes with a `"type"` discriminant field.

```mesh
type Shape do
  Circle(Float)
  Rectangle(Float, Float)
end deriving(Json)
```

## Associated Types
1. Declared in the interface: `type Item`.
2. Referred to in method signatures as `Self.Item`.
3. Assigned in the impl: `type Item = ConcreteType`.
4. Enables generic interfaces whose output type varies per implementation.
