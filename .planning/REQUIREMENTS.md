# Requirements: Mesh

**Defined:** 2026-02-28
**Milestone:** v14.0 Ecosystem & Standard Library
**Core Value:** Expressive, readable concurrency — writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.

## v14.0 Requirements

### Crypto (CRYPTO)

- [ ] **CRYPTO-01**: User can compute SHA-256 hash of a string (`Crypto.sha256(s)`) returning a lowercase hex string
- [ ] **CRYPTO-02**: User can compute SHA-512 hash of a string (`Crypto.sha512(s)`) returning a lowercase hex string
- [ ] **CRYPTO-03**: User can compute HMAC-SHA256 of a message with a key (`Crypto.hmac_sha256(key, msg)`) returning a hex string
- [ ] **CRYPTO-04**: User can compute HMAC-SHA512 of a message with a key (`Crypto.hmac_sha512(key, msg)`) returning a hex string
- [ ] **CRYPTO-05**: User can perform constant-time string comparison (`Crypto.secure_compare(a, b)`) returning Bool, safe for HMAC token verification
- [ ] **CRYPTO-06**: User can generate a UUID v4 string (`Crypto.uuid4()`) as a cryptographically random 128-bit UUID

### Encoding (ENCODE)

- [ ] **ENCODE-01**: User can base64-encode a string with standard alphabet (`Base64.encode(s)`) returning String
- [ ] **ENCODE-02**: User can base64-decode a standard-alphabet string (`Base64.decode(s)`) returning Result<String, String>
- [ ] **ENCODE-03**: User can base64-encode with URL-safe alphabet (`Base64.encode_url(s)`) returning String
- [ ] **ENCODE-04**: User can base64-decode a URL-safe string (`Base64.decode_url(s)`) returning Result<String, String>
- [ ] **ENCODE-05**: User can hex-encode a string (`Hex.encode(s)`) returning a lowercase hex string
- [ ] **ENCODE-06**: User can hex-decode a hex string (`Hex.decode(s)`) returning Result<String, String>

### DateTime (DTIME)

- [ ] **DTIME-01**: User can get the current UTC datetime via `DateTime.utc_now()` returning a DateTime value
- [ ] **DTIME-02**: User can parse an ISO 8601 string into a DateTime via `DateTime.from_iso8601(s)` returning Result<DateTime, String>
- [ ] **DTIME-03**: User can format a DateTime as an ISO 8601 string via `DateTime.to_iso8601(dt)` returning String
- [ ] **DTIME-04**: User can convert a Unix timestamp Int to DateTime via `DateTime.from_unix(n)`
- [ ] **DTIME-05**: User can convert a DateTime to a Unix timestamp Int via `DateTime.to_unix(dt)`
- [ ] **DTIME-06**: User can add a duration to a DateTime via `DateTime.add(dt, n, unit)` with units :second/:minute/:hour/:day
- [ ] **DTIME-07**: User can compute the signed difference between two DateTimes via `DateTime.diff(dt1, dt2, unit)` returning Int
- [ ] **DTIME-08**: User can compare two DateTimes via `DateTime.before?(dt1, dt2)` and `DateTime.after?(dt1, dt2)` returning Bool

### HTTP Client (HTTP)

- [ ] **HTTP-01**: User can create an HTTP request with a fluent builder via `Http.build(:get/:post/:put/:delete, url)` returning a Request value
- [ ] **HTTP-02**: User can add a header to an HTTP request via `Http.header(req, key, value)` returning an updated Request
- [ ] **HTTP-03**: User can set the request body via `Http.body(req, s)` returning an updated Request
- [ ] **HTTP-04**: User can set a per-request timeout via `Http.timeout(req, ms)` returning an updated Request
- [ ] **HTTP-05**: User can execute an HTTP request via `Http.send(req)` returning `Result<Response, String>` with status, body, and headers
- [ ] **HTTP-06**: User can stream an HTTP response chunk-by-chunk via `Http.stream(req, fn chunk -> ... end)` without buffering the full body in memory
- [ ] **HTTP-07**: User can create a keep-alive HTTP client handle via `Http.client()` and reuse connections via `Http.send_with(client, req)`

### Testing Framework (TEST)

- [ ] **TEST-01**: User can run all `*.test.mpl` files in a project via `meshc test` with a pass/fail summary per test function
- [ ] **TEST-02**: User can assert a boolean expression in a test via `assert expr` with failure output showing the expression source and value
- [ ] **TEST-03**: User can assert equality in a test via `assert_eq a, b` with expected vs actual output on failure
- [ ] **TEST-04**: User can assert inequality in a test via `assert_ne a, b` with a descriptive failure message
- [ ] **TEST-05**: User can assert that a function raises an error via `assert_raises fn`
- [ ] **TEST-06**: User can group related tests via `describe "..." do ... end` blocks with the group name shown in failure output
- [ ] **TEST-07**: User can define shared setup and teardown for a describe block via `setup do ... end` and `teardown do ... end`
- [ ] **TEST-08**: User can spawn a mock actor in a test via `Test.mock_actor(fn msg -> ... end)` returning a Pid for concurrency testing
- [ ] **TEST-09**: User can assert the test actor receives a message matching a pattern via `assert_receive pattern, timeout`
- [ ] **TEST-10**: User can generate a test coverage report via `meshc test --coverage`

### Package Manifest & CLI (PKG)

- [ ] **PKG-01**: User can declare a Mesh package in `mesh.toml` with name, version, description, license, and a dependencies section
- [ ] **PKG-02**: User project gets a `mesh.lock` lockfile auto-generated by `meshpkg install` ensuring reproducible builds
- [ ] **PKG-03**: User can publish a package to the registry via `meshpkg publish` with an auth token
- [ ] **PKG-04**: User can install a package by name via `meshpkg install <name>` downloading and extracting from the hosted registry
- [ ] **PKG-05**: User can search the registry via `meshpkg search <query>` and see matching package names and descriptions
- [ ] **PKG-06**: User can authenticate with the registry via `meshpkg login` storing a token in `~/.mesh/credentials`

### Package Registry (REG)

- [ ] **REG-01**: Registry accepts package publications via authenticated HTTP API with SHA-256 content addressing and rejects duplicate version uploads
- [ ] **REG-02**: User can browse all published packages on the hosted site listed by recency and/or popularity
- [ ] **REG-03**: User can search the hosted site for packages by name or keyword with relevant results
- [ ] **REG-04**: User can view a per-package page with rendered README, version history, and the install command

## Future Requirements (v14.1+)

### DateTime Extended

- **DTIME-09**: User can format a DateTime with a custom strftime-style pattern via `DateTime.format(dt, pattern)`
- **DTIME-10**: User can shift a DateTime to a named timezone via `DateTime.shift_zone(dt, tz)` using the IANA timezone database

### Crypto Extended

- **CRYPTO-07**: User can hash a password with PBKDF2 via `Crypto.pbkdf2(password, salt, iterations)` returning a hex string

### Testing Extended

- **TEST-11**: User can see per-file and per-function coverage percentages in `meshc test --coverage` report with delta from baseline
- **TEST-12**: User can run test modules in parallel via `meshc test --jobs N`

### Package Extended

- **PKG-07**: User can view outdated dependencies via `meshpkg outdated` comparing mesh.lock against current registry versions
- **PKG-08**: User can yank a published version via `meshpkg yank <name>@<version>` marking it deprecated without deleting it

## Out of Scope

| Feature | Reason |
|---------|--------|
| MD5 / SHA-1 hashing | Cryptographically broken; stdlib inclusion normalizes insecure use |
| Async/Future-based HTTP streaming | Contradicts Mesh's actor model; colored functions are explicitly out of scope |
| Global function replacement mocking | Breaks test isolation in parallel test runs; actor-based mocking is the pattern |
| SemVer range solving (`^1.0`, `~> 1.2`) | Significant solver complexity; exact versions sufficient for v14.0 |
| IANA full timezone database | 50+ KB binary bloat; UTC + Unix timestamps cover most use cases |
| Private/org package namespaces | Enterprise concern; not a v1 registry goal |
| Ed25519 / RSA signing | Specialized crypto beyond stdlib scope |
| `meshpkg audit` vulnerability scanning | Requires CVE database integration |

## Traceability

Which phases cover which requirements.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CRYPTO-01 | Phase 135 | Pending |
| CRYPTO-02 | Phase 135 | Pending |
| CRYPTO-03 | Phase 135 | Pending |
| CRYPTO-04 | Phase 135 | Pending |
| CRYPTO-05 | Phase 135 | Pending |
| CRYPTO-06 | Phase 135 | Pending |
| ENCODE-01 | Phase 135 | Pending |
| ENCODE-02 | Phase 135 | Pending |
| ENCODE-03 | Phase 135 | Pending |
| ENCODE-04 | Phase 135 | Pending |
| ENCODE-05 | Phase 135 | Pending |
| ENCODE-06 | Phase 135 | Pending |
| DTIME-01 | Phase 136 | Pending |
| DTIME-02 | Phase 136 | Pending |
| DTIME-03 | Phase 136 | Pending |
| DTIME-04 | Phase 136 | Pending |
| DTIME-05 | Phase 136 | Pending |
| DTIME-06 | Phase 136 | Pending |
| DTIME-07 | Phase 136 | Pending |
| DTIME-08 | Phase 136 | Pending |
| HTTP-01 | Phase 137 | Pending |
| HTTP-02 | Phase 137 | Pending |
| HTTP-03 | Phase 137 | Pending |
| HTTP-04 | Phase 137 | Pending |
| HTTP-05 | Phase 137 | Pending |
| HTTP-06 | Phase 137 | Pending |
| HTTP-07 | Phase 137 | Pending |
| TEST-01 | Phase 138 | Pending |
| TEST-02 | Phase 138 | Pending |
| TEST-03 | Phase 138 | Pending |
| TEST-04 | Phase 138 | Pending |
| TEST-05 | Phase 138 | Pending |
| TEST-06 | Phase 138 | Pending |
| TEST-07 | Phase 138 | Pending |
| TEST-08 | Phase 138 | Pending |
| TEST-09 | Phase 138 | Pending |
| TEST-10 | Phase 138 | Pending |
| PKG-01 | Phase 139 | Pending |
| PKG-02 | Phase 139 | Pending |
| PKG-03 | Phase 139 | Pending |
| PKG-04 | Phase 139 | Pending |
| PKG-05 | Phase 139 | Pending |
| PKG-06 | Phase 139 | Pending |
| REG-01 | Phase 140 | Pending |
| REG-02 | Phase 140 | Pending |
| REG-03 | Phase 140 | Pending |
| REG-04 | Phase 140 | Pending |

**Coverage:**
- v14.0 requirements: 47 total
- Mapped to phases: 47
- Unmapped: 0 ✓

---
*Requirements defined: 2026-02-28*
*Last updated: 2026-02-28 after roadmap creation (all 47 requirements mapped)*
