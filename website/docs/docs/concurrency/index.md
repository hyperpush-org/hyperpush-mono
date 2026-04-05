---
title: Concurrency
description: Mesh concurrency guide covering actors, mailboxes, message passing, supervision, timers, and fault-tolerant patterns.
---

# Concurrency

Mesh uses the actor model for concurrency, inspired by Erlang/Elixir. Actors are lightweight processes that communicate via message passing. They are isolated, supervised, and fault-tolerant.

> **Production backend proof:** This guide teaches the concurrency model. For the compact backend-proof handoff, start with [Production Backend Proof](/docs/production-backend-proof/). That page keeps public readers on the starter/examples-first route, points repo maintainers to [`mesher/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md) plus `bash scripts/verify-m051-s01.sh`, and keeps the retained backend-only proof behind `bash scripts/verify-m051-s02.sh`.

## The Actor Model

In Mesh, actors are independent units of computation that do not share memory. Each actor:

- Has its own **mailbox** for receiving messages
- Communicates exclusively via **message passing**
- Runs **concurrently** with other actors
- Is **isolated** -- one actor crashing does not bring down others

This model eliminates entire classes of concurrency bugs like data races, deadlocks, and shared-state corruption.

## Spawning Actors

Define an actor with the `actor` keyword and start it with `spawn`:

```mesh
actor greeter() do
  receive do
    msg -> println("actor received")
  end
end

fn main() do
  let pid = spawn(greeter)
  send(pid, 1)
  println("main done")
end
```

The `spawn` function returns a **PID** (process identifier) that you use to communicate with the actor. Actors run concurrently with the function that spawned them.

## Message Passing

Actors communicate by sending and receiving messages. Use `send` to deliver a message to an actor's mailbox, and `receive` to wait for and pattern match on incoming messages:

```mesh
actor worker() do
  receive do
    msg -> println("worker done")
  end
end

fn main() do
  let w1 = spawn(worker)
  let w2 = spawn(worker)
  let w3 = spawn(worker)
  send(w1, 1)
  send(w2, 2)
  send(w3, 3)
  println("main sent all")
end
```

Key points about message passing:

- Messages are processed **one at a time** from the actor's mailbox
- `receive` blocks until a matching message arrives
- Pattern matching in `receive` blocks works just like `case` expressions
- You can spawn multiple actors and send messages to each independently

Actors can also perform computation before responding. Here is an actor that runs a function when it receives a message:

```mesh
fn count_loop(n :: Int, target :: Int) -> Int do
  if n >= target do
    n
  else
    count_loop(n + 1, target)
  end
end

actor worker() do
  receive do
    msg -> println("${count_loop(0, 100)}")
  end
end

fn main() do
  let pid = spawn(worker)
  send(pid, 1)
end
```

## Linking and Monitoring

Actors can be linked so that failures propagate between them. If one linked actor crashes, the other is notified:

```mesh
actor linked_worker() do
  receive do
    msg -> println("linked worker done")
  end
end

actor linker() do
  receive do
    msg -> println("linker done")
  end
end

fn main() do
  let w = spawn(linked_worker)
  let l = spawn(linker)
  send(w, 1)
  send(l, 1)
  println("link test done")
end
```

- **`link(pid)`** -- bidirectionally links two actors. If one dies, the other receives an exit signal.
- **`monitor(pid)`** -- unidirectionally monitors an actor. The monitoring actor receives a notification if the monitored actor dies, but not vice versa.

Linking is the foundation for building fault-tolerant systems: supervisors use links to detect and restart failed actors.

## Supervision

Supervisors are special actors that monitor and restart child actors when they fail. Define a supervisor with the `supervisor` keyword:

```mesh
actor worker() do
  receive do
    msg -> println("worker got message")
  end
end

supervisor WorkerSup do
  strategy: one_for_one
  max_restarts: 3
  max_seconds: 5

  child w1 do
    start: fn -> spawn(worker) end
    restart: permanent
    shutdown: 5000
  end
end

fn main() do
  let sup = spawn(WorkerSup)
  println("supervisor started")
end
```

### Supervision Strategies

| Strategy | Behavior |
|----------|----------|
| `one_for_one` | Only the failed child is restarted |
| `one_for_all` | All children are restarted when one fails |
| `rest_for_one` | The failed child and all children started after it are restarted |

### Child Specifications

Each `child` block configures how the supervisor manages that actor:

| Option | Purpose |
|--------|---------|
| `start` | Function that spawns the child actor |
| `restart` | Restart policy: `permanent` (always), `transient` (only on abnormal exit), `temporary` (never) |
| `shutdown` | Milliseconds to wait for graceful shutdown |

### Restart Limits

- **`max_restarts`** -- maximum number of restarts allowed within the time window
- **`max_seconds`** -- the time window in seconds

If a child exceeds the restart limit, the supervisor itself shuts down, escalating the failure to its parent supervisor.

## Services (GenServer)

Services are stateful actors that follow the GenServer pattern. They provide a structured way to manage state with synchronous calls and asynchronous casts:

```mesh
service Counter do
  fn init(start_val :: Int) -> Int do
    start_val
  end

  call GetCount() :: Int do |count|
    (count, count)
  end

  call Increment(amount :: Int) :: Int do |count|
    (count + amount, count + amount)
  end

  cast Reset() do |_count|
    0
  end
end

fn main() do
  let pid = Counter.start(10)
  let c1 = Counter.get_count(pid)
  println("${c1}")
  let c2 = Counter.increment(pid, 5)
  println("${c2}")
  Counter.reset(pid)
  let c3 = Counter.get_count(pid)
  println("${c3}")
end
```

### Service Anatomy

- **`init`** -- called when the service starts, returns the initial state
- **`call`** -- synchronous request/response. The handler receives the current state and returns a tuple `(reply, new_state)`
- **`cast`** -- asynchronous fire-and-forget. The handler receives the current state and returns the new state

### Starting and Calling Services

The compiler auto-generates snake_case methods from your PascalCase definitions:

```mesh
service Store do
  fn init(start_val :: Int) -> Int do
    start_val
  end

  call Get() :: Int do |state|
    (state, state)
  end

  call Set(value :: Int) :: Int do |_state|
    (value, value)
  end

  cast Clear() do |_state|
    0
  end
end

fn main() do
  let pid = Store.start(100)
  let v1 = Store.get(pid)
  println("${v1}")
  let v2 = Store.set(pid, 200)
  println("${v2}")
  Store.clear(pid)
  let v3 = Store.get(pid)
  println("${v3}")
end
```

| Definition | Generated method |
|------------|-----------------|
| `Store.start(100)` | Starts the service with initial value |
| `Store.get(pid)` | Calls the `Get` handler |
| `Store.set(pid, 200)` | Calls the `Set` handler |
| `Store.clear(pid)` | Casts the `Clear` handler |

Services with no init arguments use `start()` with no parameters:

```mesh
service Accumulator do
  fn init() -> Int do
    0
  end

  call Add(n :: Int) :: Int do |state|
    (state + n, state + n)
  end
end

fn main() do
  let pid = Accumulator.start()
  let _ = Accumulator.add(pid, 1)
  let _ = Accumulator.add(pid, 2)
  let result = Accumulator.add(pid, 3)
  println("${result}")
end
```

## Next Steps

- [Type System](/docs/type-system/) -- structs, generics, traits, and deriving
- [Syntax Cheatsheet](/docs/cheatsheet/) -- quick reference for all Mesh syntax
