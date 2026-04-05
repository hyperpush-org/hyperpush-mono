---
title: Distributed Actors
description: Node connections, remote actors, and global process registry in Mesh
---

# Distributed Actors

> **Clustered proof surfaces:** This guide stays on the language/runtime primitives. Start with [Clustered Example](/docs/getting-started/clustered-example/) when you want the scaffold-first app route, use [Distributed Proof](/docs/distributed-proof/) when you need the named clustered verifier map, historical compatibility aliases, the S07 clustered-route rail, or the read-only Fly path, and use [Production Backend Proof](/docs/production-backend-proof/) before any deeper maintainer-only backend surface when the work becomes backend-specific. That proof page keeps the public route scaffold/examples-first, then points repo maintainers to [`mesher/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md) plus `bash scripts/verify-m051-s01.sh`, with the retained backend-only proof kept behind `bash scripts/verify-m051-s02.sh`. Keep the public scaffold/examples-first split honest here too: [`examples/todo-sqlite/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) is the honest local single-node starter with no `work.mpl`, `HTTP.clustered(...)`, or `meshc cluster` story, while [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) is the shared/deployable starter that keeps source-first `@cluster` work and only dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`. If you are migrating older clustered code, move `clustered(work)` into source-first `@cluster`, delete any `[cluster]` manifest stanza, rename helper-shaped entries such as `execute_declared_work(...)` / `Work.execute_declared_work` to ordinary verbs like `add()` or `sync_todos()`, and let the runtime-owned CLI carry operator truth instead of recreating package-owned control routes here.

Mesh's actor model extends seamlessly across machines. The same primitives you use locally -- `spawn`, `send`, `receive` -- work across networked nodes. Once two nodes are connected, processes on either side can communicate transparently.

Distribution is built on TLS-encrypted TCP connections with cookie-based authentication. Every node that joins a cluster must share the same secret cookie, which is verified via an HMAC-SHA256 challenge/response handshake.

## Starting a Node

A Mesh runtime becomes a named, addressable node by calling `Node.start`. This binds a TCP listener and makes the process ready to accept connections from other nodes:

```mesh
fn main() do
  Node.start("app@localhost:4000", "secret_cookie")
  println("Node started")
end
```

The first argument is the node name in `"name@host:port"` format. The second argument is the shared secret cookie used for authentication. All nodes in a cluster must use the same cookie.

Behind the scenes, `Node.start`:
1. Parses the node address and binds a TCP listener on the given port
2. Generates an ephemeral TLS certificate for encrypted communication
3. Starts an accept loop to handle incoming connections from other nodes

## Connecting Nodes

Once a node is started, it can connect to other nodes with `Node.connect`:

```mesh
fn main() do
  Node.start("app@localhost:4000", "my_cookie")
  Node.connect("worker@localhost:4001")
  println("Connected to worker")
end
```

When two nodes connect, they perform a mutual cookie handshake. If either side has a different cookie, the connection is rejected. After authentication, both nodes exchange their global registry state to synchronize cluster-wide process names.

### Querying the Cluster

You can inspect the cluster state with `Node.self` and `Node.list`:

```mesh
fn main() do
  Node.start("app@localhost:4000", "my_cookie")
  Node.connect("worker@localhost:4001")

  let me = Node.self()
  println("I am: ${me}")

  let nodes = Node.list()
  println("Connected nodes: ${nodes}")
end
```

| Function | Description |
|----------|-------------|
| `Node.self()` | Returns the name of the current node |
| `Node.list()` | Returns a list of all connected node names |

## Remote Actors

Once nodes are connected, you can spawn actors on remote nodes and communicate with them using the same `send` and `receive` primitives you use locally.

### Spawning on a Remote Node

Use `Node.spawn` to start an actor on a specific remote node:

```mesh
actor worker() do
  receive do
    msg -> println("Remote worker got: ${msg}")
  end
end

fn main() do
  Node.start("app@localhost:4000", "my_cookie")
  Node.connect("worker@localhost:4001")

  let pid = Node.spawn("worker@localhost:4001", worker)
  send(pid, "hello from app node")
end
```

`Node.spawn` returns a PID that is valid across nodes. Sending a message to this PID routes it over the network to the remote node transparently.

### Spawning with Links

Use `Node.spawn_link` to spawn a remote actor and establish a bidirectional link in one step. If either the local or remote actor crashes, the other receives an exit signal:

```mesh
actor task() do
  receive do
    msg -> println("task completed")
  end
end

fn main() do
  Node.start("app@localhost:4000", "my_cookie")
  Node.connect("worker@localhost:4001")

  let pid = Node.spawn_link("worker@localhost:4001", task)
  send(pid, "start")
end
```

This is the distributed equivalent of `spawn_link` -- it combines spawning and linking into a single atomic operation.

## Global Registry

The global registry provides cluster-wide process name registration. Unlike local process names (which are scoped to a single node), global names are replicated across all connected nodes.

### Registering a Name

Use `Global.register` to assign a name to a process globally:

```mesh
fn main() do
  Node.start("app@localhost:4000", "my_cookie")

  Global.register("db_service", self())
  println("Registered as db_service")
end
```

When a name is registered, it is broadcast to all connected nodes. Every node holds a complete replica of the name table, so lookups are always local (no network round-trip).

### Looking Up a Name

Use `Global.whereis` to find a process by its global name:

```mesh
fn main() do
  Node.start("app@localhost:4000", "my_cookie")
  Node.connect("db@localhost:4001")

  let pid = Global.whereis("db_service")
  send(pid, "query")
end
```

Since every node has a full replica of the global registry, `Global.whereis` returns immediately without any network call.

### Unregistering a Name

Use `Global.unregister` to remove a global registration:

```mesh
fn main() do
  Node.start("app@localhost:4000", "my_cookie")

  Global.register("temp_worker", self())
  # ... do some work ...
  Global.unregister("temp_worker")
end
```

| Function | Description |
|----------|-------------|
| `Global.register(name, pid)` | Register a process globally across all nodes |
| `Global.whereis(name)` | Look up a globally registered process by name |
| `Global.unregister(name)` | Remove a global name registration |

### Automatic Cleanup

The global registry automatically cleans up registrations when:

- A **process exits** -- all global names registered by that process are removed
- A **node disconnects** -- all global names owned by that node are removed

This means you do not need to manually unregister names in crash or disconnect scenarios. The cleanup is broadcast to all remaining nodes in the cluster.

## Node Monitoring

You can monitor remote nodes to receive notifications when they disconnect:

```mesh
actor watcher() do
  receive do
    (:nodedown, name) -> println("Node disconnected: ${name}")
  end
end

fn main() do
  Node.start("app@localhost:4000", "my_cookie")
  Node.connect("worker@localhost:4001")

  Node.monitor("worker@localhost:4001")

  # If worker disconnects, the current process receives a :nodedown message
end
```

`Node.monitor` sets up a notification so that the calling process receives a `:nodedown` tuple when the monitored node disconnects. This is the distributed equivalent of process monitoring -- instead of watching a single process, you watch an entire node.

## API Reference

| Module | Function | Description |
|--------|----------|-------------|
| `Node` | `start(name, cookie)` | Start a named node with cookie authentication |
| `Node` | `connect(name)` | Connect to a remote node |
| `Node` | `self()` | Get the current node name |
| `Node` | `list()` | List all connected nodes |
| `Node` | `spawn(node, actor)` | Spawn an actor on a remote node |
| `Node` | `spawn_link(node, actor)` | Spawn a linked actor on a remote node |
| `Node` | `monitor(name)` | Monitor a remote node for disconnection |
| `Global` | `register(name, pid)` | Register a process name globally |
| `Global` | `whereis(name)` | Look up a global process name |
| `Global` | `unregister(name)` | Remove a global name registration |

## Next Steps

- [Distributed Proof](/docs/distributed-proof/) -- the canonical public proof surface for the scaffold-first clustered-app/operator story, the bounded failover/operator rail, and the read-only Fly evidence path
- [Concurrency](/docs/concurrency/) -- actors, supervision, and services on a single node
- [Developer Tools](/docs/tooling/) -- formatter, REPL, package manager, and editor support
