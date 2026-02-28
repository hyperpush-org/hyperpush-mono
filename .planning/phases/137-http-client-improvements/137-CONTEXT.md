# Phase 137: HTTP Client Improvements - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Mesh programs can make HTTP requests with a fluent builder API, stream large responses without buffering the full body, and reuse connections via a keep-alive agent handle. This phase covers the builder API (Http.build/header/body/query/json/timeout/send), streaming (Http.stream / Http.stream_bytes), and keep-alive client handle (Http.client / Http.send_with). Creating new HTTP server endpoints and transport-layer changes are out of scope.

</domain>

<decisions>
## Implementation Decisions

### Builder API design
- `Http.build(:get, url)` — method is an atom (`:get`, `:post`, `:put`, `:delete`, etc.)
- Builder helpers: `Http.header(req, k, v)`, `Http.body(req, s)`, `Http.timeout(req, ms)`, `Http.query(req, key, val)`, `Http.json(req, body)`
- `Http.json` sets Content-Type to application/json and serializes the body
- `Http.query` appends `?key=val` to the URL
- Response is a struct with fields: `resp.status` (Int), `resp.body` (String), `resp.headers` (Map<String, String>)
- `resp.headers` uses Mesh's native `Map<String, String>` type (existing `%{key => value}` syntax and `Map` module)

### Error model
- `Ok(resp)` for all valid HTTP responses regardless of status code; user checks `resp.status` to determine success/failure
- `Err(String)` only for network-level failures (timeout, DNS, TLS, connection refused)
- Error string format: `"ERROR_CODE: human message"` — e.g., `"TIMEOUT: connection timed out after 5000ms"`, `"DNS_FAILURE: could not resolve example.com"`, `"TLS_ERROR: certificate verification failed"`
- Redirect following: automatic (Claude decides redirect limit)
- TLS certificate errors: always fail — no opt-out mechanism

### Keep-alive client lifecycle
- `Http.client()` returns a per-actor opaque handle — each actor can have its own connection pool
- Client is also a builder: `Http.client() |> Http.base_url(c, url) |> Http.default_header(c, k, v)`
- Automatic cleanup when the handle goes out of scope or the actor dies (Rust Drop trait binding)
- `Http.send_with(client, req)` sends a request using the client's connection pool
- Request-level config overrides client-level defaults (headers merged, request wins on conflict; timeout, URL fully overridden by request if set)

### Streaming behavior
- Two stream functions:
  - `Http.stream(req, fn chunk -> ... end)` — String chunks (UTF-8 decoded)
  - `Http.stream_bytes(req, fn chunk -> ... end)` — raw byte chunks (for binary content)
- OS-thread-per-stream model (one OS thread per active stream)
- Both functions return `Result<Response, String>` — response carries status and headers but empty body; `Err` on network failure
- Two cancellation mechanisms:
  1. **Inline**: callback returns `:stop` to abort the stream
  2. **External**: `Http.stream`/`Http.stream_bytes` returns a cancel handle; caller can call `Http.cancel(handle)` from another actor

### Claude's Discretion
- Connection pool size per client handle
- Exact chunk size from the underlying ureq/Rust layer
- How `:stop` propagates through the OS-thread boundary
- Compression (gzip) support (auto or opt-in)
- Whether `Http.json` requires a prior JSON encoding phase or does inline serialization

</decisions>

<specifics>
## Specific Ideas

- The pipe-forward style `Http.build(:get, url) |> Http.header(req, "Authorization", "Bearer token") |> Http.send(req)` is the canonical usage pattern (from success criteria)
- `Http.client()` should mirror the request builder pattern for consistency — users already know the pipe-forward idiom from request building

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 137-http-client-improvements*
*Context gathered: 2026-02-28*
