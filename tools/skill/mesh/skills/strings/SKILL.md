---
name: mesh-strings
description: Mesh string features: interpolation (#{} and ${}), heredocs, json { } object literals, String stdlib, Env.get/get_int, and the Regex module.
---

## String Interpolation

Rules:
1. `"text #{expr} text"` — hash-brace `#{}` is the primary interpolation syntax.
2. Any expression is valid inside `#{}`: variables, function calls, arithmetic.
3. `"text ${expr}"` — dollar-brace `${}` also works (legacy syntax, fully supported).
4. The expression is evaluated and converted to String via `.to_string()` automatically.
5. Nested quotes in interpolated expressions do not require escaping.

Code example (from tests/e2e/string_interp_hash.mpl and tests/e2e/comprehensive.mpl):
```mesh
let name = "Alice"
let age = 30
println("Hello, #{name}! You are #{age} years old.")

let result = add(10, 20)
println("Sum: #{result}")
println("Double: #{double(7)}")
```

## Heredoc Strings

Rules:
1. `"""..."""` — triple-quoted heredoc for multiline strings.
2. Content preserves all whitespace and newlines.
3. Common indentation is stripped (trimIndent behavior) — align content with the closing `"""`.
4. Heredocs support `#{}` interpolation just like regular strings.
5. Use for JSON bodies, multi-line SQL, or any string with embedded double-quotes.

Code example (from tests/e2e/heredoc_interp.mpl):
```mesh
let id = 42
let name = "Alice"
let body = """
  {"id": #{id}, "name": "#{name}"}
  """
println(body)
# Outputs: {"id": 42, "name": "Alice"}
```

> **For JSON objects:** Prefer `json { }` literals over heredoc JSON templates — they are type-safe and require no manual escaping. See **JSON Literals** below.

## JSON Literals

Rules:
1. `json { key: value, ... }` — constructs a JSON object. Keys are bare identifiers; values are any Mesh expression.
2. Multi-line syntax works identically — newlines inside `{ }` are insignificant.
3. The result is a `Json` type that auto-coerces to `String` — pass directly to `HTTP.response`, `Ws.broadcast`, or any function expecting a String.
4. Nesting: assign `json { }` to a variable then use it as a field value — embedded raw, no double-encoding.
5. Reserved keywords (`type`, `fn`, `let`, etc.) cannot be used as bare keys — use heredoc strings for those cases.

Type serialization table:

| Mesh type | JSON output |
|-----------|-------------|
| `String`  | `"quoted string"` |
| `Int`     | `42` (unquoted number) |
| `Float`   | `3.14` (unquoted) |
| `Bool`    | `true` / `false` |
| `nil`     | `null` |
| `Option<T>` | `null` (None) or the serialized value (Some) |
| `List<T>` | JSON array |
| Struct with `deriving(Json)` | nested JSON object |

Code example (from tests/e2e/json_literal_basic.mpl and phase 132 Mesher migration):
```mesh
# Basic object — auto-coerces to String
HTTP.response(200, json { status: "ok", count: 42 })
HTTP.response(401, json { error: "unauthorized" })

# Multi-line
let event = json {
  issue_id: issue_id,
  severity: "high",
  active: true
}

# Nesting — inner embedded raw, no double-encoding
let inner = json { code: 200 }
let outer = json { result: inner, ok: true }
# outer is: {"result":{"code":200},"ok":true}

# Option field
let data = json { value: some_option }  # None -> null, Some(v) -> v

# List<String> as array
let tags = ["a", "b", "c"]
let obj = json { tags: tags }  # {"tags":["a","b","c"]}
```

## String Stdlib

Rules (String module functions):
1. `String.length(s)` — character count.
2. `String.contains(s, substring)` — returns Bool.
3. `String.starts_with(s, prefix)` / `String.ends_with(s, suffix)` — prefix/suffix check.
4. `String.split(s, delimiter)` — returns `List<String>`.
5. `String.join(list, separator)` — joins list of strings with separator.
6. `String.trim(s)` / `String.trim_start(s)` / `String.trim_end(s)` — whitespace stripping.
7. `String.to_upper(s)` / `String.to_lower(s)` — case conversion.
8. `String.replace(s, from, to)` — literal string replacement.
9. `String.parse(s)` — attempts conversion (returns `Option<T>` based on context).
10. `s.to_string()` — any value has this method (via Display trait).
11. String concatenation: `s1 <> s2` operator.

Code example (from tests/e2e/stdlib_string_split_join.mpl):
```mesh
let words = String.split("hello world foo", " ")
let joined = String.join(words, "-")
println(joined)   # "hello-world-foo"
```

## Env Variables

Rules:
1. `Env.get("VAR_NAME", "default")` — reads env var as String; returns default if unset.
2. `Env.get_int("PORT", 8080)` — reads env var as Int; returns default if unset or non-numeric.
3. Both functions always return a value — no error handling needed.
4. The default is returned for: unset vars, empty vars (Env.get), and non-integer values (Env.get_int).

Code example (from tests/e2e/env_get.mpl and tests/e2e/env_get_int.mpl):
```mesh
let port = Env.get_int("PORT", 8080)
let host = Env.get("HOST", "0.0.0.0")
println("Listening on #{host}:#{port}")
```

## Regex

Rules:
1. Regex literals: `~r/pattern/` and `~r/pattern/flags` (flags: i=case insensitive, m=multiline, s=dotall).
2. Runtime compile: `Regex.compile(str) -> Result<Regex, String>`.
3. `Regex.is_match(rx, str) -> Bool` — tests whether pattern matches.
4. `Regex.captures(rx, str) -> Option<List<String>>` — returns capture groups (index 0 = first group).
5. `Regex.replace(rx, str, replacement) -> String` — replaces all matches.
6. `Regex.split(rx, str) -> List<String>` — splits by pattern.
7. Regex literals are compiled at compile time — prefer them over `Regex.compile` for static patterns.

Code example (from tests/e2e/regex_literal.mpl and tests/e2e/regex_captures.mpl):
```mesh
# Literal (compile-time):
let rx = ~r/\d+/
println("#{Regex.is_match(rx, "hello123")}")   # true

# Case-insensitive flag:
let rx2 = ~r/[a-z]+/i
println("#{Regex.is_match(rx2, "HELLO")}")     # true

# Captures:
let rx3 = ~r/(\w+)\s(\w+)/
case Regex.captures(rx3, "hello world") do
  Some(caps) -> println(List.head(caps))       # "hello"
  None -> println("no match")
end

# Replace and split:
let rx4 = ~r/\s+/
let normalized = Regex.replace(rx4, "hello   world", " ")
let parts = Regex.split(rx4, "one  two   three")
```

Code example — runtime compile (from tests/e2e/regex_compile.mpl):
```mesh
let pattern = "\\d+"
case Regex.compile(pattern) do
  Ok(rx) ->
    let found = Regex.is_match(rx, "abc123")
    println("#{found}")
  Err(e) -> println("invalid pattern: #{e}")
end
```
