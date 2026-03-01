---
name: mesh-http
description: Mesh HTTP: server routing (HTTP.router/route/serve), middleware (HTTP.use), path parameters, HTTP client v14 fluent builder (Http.build/send/stream), legacy HTTP.get, and WebSocket.
---

## HTTP Server Basics

Rules:
1. `HTTP.router()` creates a new router instance.
2. `HTTP.route(router, path, handler)` registers a handler for a path — returns updated router (rebind).
3. `HTTP.serve(router, port)` starts the HTTP server on the given port (blocks).
4. Handler signature: `fn handler(request) do ... HTTP.response(status, body) end`.
5. `HTTP.response(status_code, body_string)` creates a Response — return this from handlers.
6. For JSON responses, use `json { }` literals — they are type-safe and auto-coerce to String: `HTTP.response(200, json { status: "ok", id: record_id })`.

Code example (from tests/e2e/stdlib_http_server_runtime.mpl):
```mesh
fn handler(request) do
  HTTP.response(200, json { status: "ok" })
end

fn main() do
  let r = HTTP.router()
  let r = HTTP.route(r, "/health", handler)
  HTTP.serve(r, 8080)
end
```

## Request Object

Rules:
1. Handlers receive a `Request` value — use `Request.*` functions to inspect it.
2. `Request.path(request)` — returns the URL path as String.
3. `Request.param(request, "name")` — extracts a named path parameter (e.g., from `/users/:id`).
4. `Request.body(request)` — returns the raw request body as String.
5. `Request.header(request, "Header-Name")` — reads a request header.

Code example (from tests/e2e/stdlib_http_path_params.mpl):
```mesh
fn user_handler(request :: Request) do
  let id = Request.param(request, "id")
  HTTP.response(200, "user id: #{id}")
end

fn main() do
  let r = HTTP.router()
  let r = HTTP.route(r, "/users/:id", user_handler)
  HTTP.serve(r, 8080)
end
```

## Response Helpers

Rules:
1. `HTTP.response(status, body)` — creates response with given status code and string body.
2. Return this from any handler or middleware.
3. Common status codes: 200 (ok), 201 (created), 400 (bad request), 401 (unauthorized), 404 (not found), 500 (error).

Code example (from tests/e2e/stdlib_http_response.mpl):
```mesh
fn handler(request :: Request) -> Response do
  HTTP.response(200, "hello world")
end
```

## Middleware

Rules:
1. `HTTP.use(router, middleware_fn)` registers middleware — returns updated router (rebind).
2. Middleware signature: `fn name(request :: Request, next) -> Response do ... end`.
3. Call `next(request)` to pass control to the next middleware or final handler.
4. Return a Response directly (without calling `next`) to short-circuit (e.g., auth rejection).
5. Middleware executes in registration order — first registered runs first.
6. Multiple middleware can be chained with successive `HTTP.use` calls.

Code example (from tests/e2e/stdlib_http_middleware.mpl):
```mesh
fn logger(request :: Request, next) -> Response do
  next(request)   # pass through
end

fn auth_check(request :: Request, next) do
  let path = Request.path(request)
  if String.starts_with(path, "/secret") do
    HTTP.response(401, "Unauthorized")   # short-circuit
  else
    next(request)
  end
end

fn main() do
  let r = HTTP.router()
  let r = HTTP.use(r, logger)
  let r = HTTP.use(r, auth_check)
  let r = HTTP.route(r, "/hello", handler)
  let r = HTTP.route(r, "/secret", secret_handler)
  HTTP.serve(r, 8080)
end
```

## HTTP Client

Rules:
1. `HTTP.get(url) -> Result<String, String>` — synchronous GET; returns body or error message.
2. Returns `Ok(body_string)` on 2xx, `Err(message)` on failure.
3. Use the `?` operator or `case` to handle the result.

Code example (from tests/e2e/stdlib_http_client.mpl):
```mesh
let result = HTTP.get("http://api.example.com/data")
case result do
  Ok(body) -> println(body)
  Err(msg) -> println("request failed: #{msg}")
end
```

## HTTP Client v14 (Builder API)

Rules:
1. `Http.build(method, url)` — creates a new request. `method` is an atom: `:get`, `:post`, `:put`, `:delete`. Returns a Request handle.
2. `Http.header(req, key, value)` — adds a header. Returns updated request (rebind).
3. `Http.body(req, s)` — sets the request body string (for POST/PUT). Returns updated request.
4. `Http.timeout(req, ms)` — sets a per-request timeout in milliseconds. Returns updated request.
5. `Http.send(req)` — executes the request. Returns `Result<String, String>` — `Ok(body)` on 2xx, `Err(message)` otherwise.
6. `Http.stream(req, fn chunk -> ... end)` — streams response body chunk by chunk in an OS thread. Callback returns `"ok"` to continue, `"stop"` to cancel. Does not buffer the full body.
7. `Http.client()` — creates a keep-alive HTTP client handle for connection reuse.
8. `Http.send_with(client, req)` — sends request using the client's connection pool. Returns `Result<String, String>`.
9. `Http.client_close(client)` — releases the client and its connections.
10. CRITICAL: `Http.*` (lowercase) = HTTP CLIENT. `HTTP.*` (uppercase) = HTTP SERVER. Never mix them.

Code example — GET with headers (from tests/e2e/http_client_builder.mpl):
```mesh
fn main() do
  let req = Http.build(:get, "https://api.example.com/data")
  let req = Http.header(req, "Authorization", "Bearer token")
  let req = Http.timeout(req, 5000)
  let result = Http.send(req)
  case result do
    Ok(resp) -> println(resp)
    Err(e) -> println("error: #{e}")
  end
end
```

Code example — POST with body (from tests/e2e/http_client_builder.mpl):
```mesh
fn main() do
  let req = Http.build(:post, "https://api.example.com/items")
  let req = Http.header(req, "Content-Type", "application/json")
  let req = Http.body(req, json { name: "widget" })
  let result = Http.send(req)
  case result do
    Ok(resp) -> println(resp)
    Err(e) -> println(e)
  end
end
```

Code example — Streaming (from tests/e2e/http_stream_compile.mpl):
```mesh
fn main() do
  let req = Http.build(:get, "https://example.com/stream")
  let _handle = Http.stream(req, fn chunk do
    println(chunk)
    "ok"
  end)
end
```

Code example — Keep-alive client (from tests/e2e/http_client_keepalive.mpl):
```mesh
fn main() do
  let client = Http.client()
  let req = Http.build(:get, "https://api.example.com/data")
  let result = Http.send_with(client, req)
  case result do
    Ok(resp) -> println(resp)
    Err(e) -> println(e)
  end
  Http.client_close(client)
end
```

## WebSocket

Rules:
1. WebSocket support is integrated with the HTTP server — same router.
2. `HTTP.ws_route(router, path, ws_handler)` registers a WebSocket handler.
3. WebSocket handlers work with the actor runtime — each connection is an actor.
4. `HTTP.ws_send(conn, message)` sends a message to a WebSocket connection.
5. See skills/actors for the actor model that backs WebSocket connections.

## Error Handling with HTTP.serve

Rules:
1. `HTTP.serve` runs the server inside the actor runtime.
2. Handler panics are isolated — one failing request does not crash the server.
3. For production use, pair with a supervisor (see skills/supervisors) to restart the server actor on crash.
4. The `HTTP.crash_isolation` test verifies that panicking handlers return 500 without killing the server.

Code example (from tests/e2e/stdlib_http_crash_isolation.mpl):
```mesh
fn panicky_handler(request :: Request) do
  panic("deliberate crash")
end

fn safe_handler(request :: Request) do
  HTTP.response(200, "ok")
end

fn main() do
  let r = HTTP.router()
  let r = HTTP.route(r, "/crash", panicky_handler)
  let r = HTTP.route(r, "/safe", safe_handler)
  HTTP.serve(r, 8085)
  # /crash returns 500; /safe still returns 200
end
```
