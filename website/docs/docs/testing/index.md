---
title: Testing
description: Write and run tests in Mesh with meshc test — assertions, grouping, mock actors, and receive expectations
---

# Testing

Mesh includes a first-class testing framework accessible via `meshc test`. Test files use the `.test.mpl` extension and can contain individual tests, grouped tests with shared setup/teardown, mock actors, and receive assertions.

## Running Tests

```bash
meshc test .              # all *.test.mpl files in the current directory
meshc test path/to/dir/  # specific directory
```

`meshc test` discovers all `*.test.mpl` files in the target directory, compiles and runs each independently, and prints a summary:

```
test arithmetic is correct ... ok
test string operations/length ... ok

2 tests, 0 failures
```

On failure, the output includes the failing assertion, the expected and actual values, and the file/test name. The exit code is non-zero if any test fails.

## Writing Tests

Test files are standalone `.test.mpl` programs. Each `test` block defines a named test:

```mesh
test("arithmetic is correct") do
  assert(1 + 1 == 2)
  assert_eq(10, 5 + 5)
  assert_ne(3, 4)
end

test("string operations") do
  assert(String.length("hello") == 5)
  assert_eq("hello", String.downcase("HELLO"))
end
```

## Assertions

| Assertion | Description |
|-----------|-------------|
| `assert expr` | Passes if `expr` is true; prints expression source and value on failure |
| `assert_eq a, b` | Passes if `a == b`; prints expected and actual on failure |
| `assert_ne a, b` | Passes if `a != b`; prints both values on failure |
| `assert_raises fn` | Passes if calling `fn` raises a runtime error |

```mesh
test("assertions") do
  assert(true)
  assert_eq(42, 40 + 2)
  assert_ne("hello", "world")
  assert_raises fn() do
    panic("intentional")
  end
end
```

## Grouping with describe

Use `describe` to group related tests. The group name appears in failure output:

```mesh
describe("string operations") do
  test("length") do
    assert(String.length("hello") == 5)
  end

  test("concat") do
    assert_eq("ab", "a" <> "b")
  end
end
```

Failed test output shows: `string operations/length ... FAIL`

## Setup and Teardown

`setup` and `teardown` blocks run before and after each test in a `describe` group:

```mesh
describe("counter") do
  setup do
    assert(true)   # runs before each test in this describe
  end

  teardown do
    assert(true)   # runs after each test in this describe
  end

  test("increments") do
    assert_eq(1, 0 + 1)
  end
end
```

`setup` and `teardown` are scoped to the `describe` block — they do not affect tests outside of it.

## Mock Actors

Use `Test.mock_actor` to spawn a lightweight actor in a test. The actor runs a callback for each message it receives:

```mesh
test("mock actor messaging") do
  let me = self()
  let mock = Test.mock_actor(fn msg do
    send(me, msg)
    "ok"
  end)
  send(mock, "hello")
  assert_receive "hello", 500
end
```

`Test.mock_actor(fn msg -> ... end)` returns a `Pid` you can `send` messages to. The mock callback must return a string — `"ok"` to continue, `"stop"` to terminate the mock.

## assert_receive

`assert_receive` waits for the current test actor to receive a message matching a pattern:

```mesh
test("receive a message") do
  let me = self()
  send(me, 42)
  assert_receive 42, 500   # pattern, timeout_ms
end
```

If the message is not received within the timeout, the test fails with the pattern and elapsed time. The default timeout is 100ms when omitted:

```mesh
assert_receive "done", 1000   # explicit timeout
```

## Coverage

Pass `--coverage` to print a stub coverage report:

```bash
meshc test --coverage .
```

> Coverage reporting is available as a stub in v14.0 — full line-level coverage will be added in v14.1.

## What's Next?

- [Standard Library](/docs/stdlib/) — Crypto, Encoding, DateTime modules
- [Developer Tools](/docs/tooling/) — meshc, meshpkg, formatter, REPL, LSP
- [Concurrency](/docs/concurrency/) — actors and supervision for testing async code
