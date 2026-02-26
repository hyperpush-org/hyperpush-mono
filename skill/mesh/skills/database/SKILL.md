---
name: mesh-database
description: Mesh database access: SQLite (Sqlite module), PostgreSQL (Pg module), and ORM patterns with deriving Row and query builder.
---

## SQLite

Rules:
1. `Sqlite.open(path) -> Result<Db, String>` — opens a database file; use `":memory:"` for in-memory.
2. `Sqlite.execute(db, sql, params) -> Result<Int, String>` — DDL and DML; returns rows affected.
3. `Sqlite.query(db, sql, params) -> Result<List<Map<String, String>>, String>` — SELECT; returns list of row maps.
4. `Sqlite.close(db)` — closes the connection.
5. Parameters are passed as `List<String>` using `?` placeholders in SQL.
6. All result map values are `String` — use `deriving(Row)` to coerce types (see below).
7. The `?` operator on results propagates errors up cleanly.

Code example (from tests/e2e/stdlib_sqlite.mpl):
```mesh
fn run_db() -> Int!String do
  let db = Sqlite.open(":memory:")?

  let _ = Sqlite.execute(db, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age TEXT)", [])?

  let _ = Sqlite.execute(db, "INSERT INTO users (name, age) VALUES (?, ?)", ["Alice", "30"])?
  let _ = Sqlite.execute(db, "INSERT INTO users (name, age) VALUES (?, ?)", ["Bob", "25"])?

  let rows = Sqlite.query(db, "SELECT name, age FROM users ORDER BY name", [])?
  List.map(rows, fn(row) do
    let name = Map.get(row, "name")
    let age = Map.get(row, "age")
    println("#{name}:#{age}")
  end)

  Sqlite.close(db)
  Ok(0)
end

fn main() do
  case run_db() do
    Ok(_) -> println("done")
    Err(msg) -> println("error: #{msg}")
  end
end
```

## PostgreSQL

Rules:
1. `Pg.connect(connection_string) -> Result<Conn, String>` — connects to PostgreSQL.
2. `Pg.execute(conn, sql, params) -> Result<Int, String>` — DDL and DML.
3. `Pg.query(conn, sql, params) -> Result<List<Map<String, String>>, String>` — SELECT.
4. `Pg.close(conn)` — closes the connection.
5. Parameters use `$1`, `$2`, ... placeholders (PostgreSQL syntax).
6. Params passed as `List<String>`.
7. Same row map pattern as SQLite: all values are String.

Code example (from tests/e2e/stdlib_pg.mpl):
```mesh
fn run_db() -> Int!String do
  let conn = Pg.connect("postgres://mesh_test:mesh_test@localhost:5432/mesh_test")?

  let _ = Pg.execute(conn, "CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT, age INTEGER)", [])?
  let _ = Pg.execute(conn, "INSERT INTO users (name, age) VALUES ($1, $2)", ["Alice", "30"])?

  let rows = Pg.query(conn, "SELECT name FROM users WHERE age > $1", ["26"])?
  List.map(rows, fn(row) do
    println(Map.get(row, "name"))
  end)

  Pg.close(conn)
  Ok(0)
end
```

## Deriving Row (ORM Pattern)

Rules:
1. `end deriving(Row)` on a struct generates `TypeName.from_row(map) -> Result<TypeName, String>`.
2. The map is a `Map<String, String>` (as returned by Sqlite.query / Pg.query).
3. Automatic type coercion: `Int` fields parse from string, `Float` from string, `Bool` from "t"/"true"/"1".
4. Field names in the struct must match column names in the query result.
5. Combine with `List.map` to convert rows to typed structs.
6. See skills/traits for the full deriving system — Row is one of several available derives.

Code example (from tests/e2e/deriving_row_basic.mpl):
```mesh
struct User do
  name :: String
  age :: Int
  score :: Float
  active :: Bool
end deriving(Row)

let rows = Sqlite.query(db, "SELECT name, age, score, active FROM users", [])?
let users = List.map(rows, fn(row) do
  case User.from_row(row) do
    Ok(u) -> u
    Err(e) -> panic("row conversion failed: #{e}")
  end
end)
```

## Advanced Queries (Upserts, RETURNING, Subqueries)

Rules:
1. Upsert with `ON CONFLICT ... DO UPDATE SET ... RETURNING` uses `Sqlite.query` (not execute) to get results.
2. `DELETE ... RETURNING` similarly uses `Sqlite.query` to capture deleted rows.
3. Subqueries are plain SQL — embed with `IN (SELECT ...)`.
4. `RETURNING` gives back the affected row data as a query result.

Code example (from tests/e2e/sqlite_upsert_subquery_runtime.mpl):
```mesh
# Upsert (insert-or-update):
let rows = Sqlite.query(db,
  "INSERT INTO projects (id, name, status) VALUES (?, ?, ?) ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name RETURNING id, name",
  ["p1", "Alpha", "active"])?
let row = List.head(rows)
println("upserted: #{Map.get(row, "name")}")

# DELETE RETURNING:
let deleted = Sqlite.query(db,
  "DELETE FROM projects WHERE id = ? RETURNING id, name",
  ["p3"])?
let del_row = List.head(deleted)
println("deleted: #{Map.get(del_row, "name")}")

# Subquery:
let results = Sqlite.query(db,
  "SELECT name FROM projects WHERE org_id IN (SELECT id FROM organizations WHERE name = ?)",
  ["Acme Corp"])?
```

## JOINs and Aggregations

Rules:
1. JOINs are plain SQL — `INNER JOIN`, `LEFT JOIN`, etc. — no special Mesh syntax.
2. Aggregations (`COUNT`, `SUM`, `AVG`, `MAX`, `MIN`) use SQL directly.
3. `GROUP BY` and `ORDER BY` are plain SQL.
4. Results are always `List<Map<String, String>>` — column aliases work as map keys.

Code example (from tests/e2e/sqlite_join_runtime.mpl):
```mesh
let rows = Sqlite.query(db,
  "SELECT u.name, COUNT(p.id) AS project_count FROM users u LEFT JOIN projects p ON u.id = p.user_id GROUP BY u.id ORDER BY u.name",
  [])?
List.map(rows, fn(row) do
  let name = Map.get(row, "name")
  let count = Map.get(row, "project_count")
  println("#{name}: #{count} projects")
end)
```

## Gotchas

Rules:
1. Always close connections with `Sqlite.close(db)` / `Pg.close(conn)` — connections are not auto-closed.
2. SQLite parameters use `?` placeholders; PostgreSQL uses `$1`, `$2`, ...
3. Row map values are always `String` — no implicit type conversion without `deriving(Row)`.
4. For `INSERT`/`UPDATE`/`DELETE` without `RETURNING`, use `execute` (not `query`).
5. For `INSERT ... RETURNING` or `DELETE ... RETURNING`, use `query` to receive the rows.
