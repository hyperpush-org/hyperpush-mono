
---

# The Sentry replacement that required inventing a language.

Most error tracking tools force the same bad choice.

Pay SaaS tax forever or self-host something that feels like punishment.

That tradeoff gets worse for small teams, open-source projects, and anyone building in public. You still need real stack traces, issue grouping, alerts, and performance visibility. But the tools that are supposed to reduce operational pain usually become another recurring bill, another black box, or another piece of infrastructure you have to babysit.

We wanted something better. So we built **Hyperpush**: an open-source, self-hostable Sentry replacement with zero lock-in, a real error tracker that small teams can adopt without asking a finance department for permission.

Not a crypto wrapper. Not a vague "observability platform." Not a pitch deck category.

An error tracker. One you can point at your app and say: *yes, this catches the crashes, shows me what broke, and helps me fix it.*

---

## Error tracking is a concurrency problem in disguise

The landing-page version of error tracking sounds simple. The real system is not.

Events arrive in bursts. Some need grouping. Some need alerting. Some need enrichment. Some need to update existing issues. Some need to appear in a live feed immediately. Some need to fan out to automations. And none of that should take the system down because one path misbehaved.

That is not CRUD with charts. That is a concurrency problem.

When we looked at what language to build Hyperpush in, we kept running into the same wall.

---

## The existing languages kept making us choose

We wanted the actor model. Supervision. Fault isolation. The concurrency story that made Erlang and Elixir legendary.

We also wanted native compiled performance, tight binaries, and a runtime we could shape around the product instead of around someone else's VM.

The usual options felt compromised. Fast languages made concurrency feel bolted on. Beautiful distributed languages lacked the compiled, systems-level shape we needed. And some were simply too unpleasant to work in for years at a stretch.

We didn't want to spend the next few years fighting the language while building the product.

So we stopped trying to pick the least-wrong option.

We built **Mesh**.

---

## Mesh: Elixir's actor model. Faster. And distribution as a first-class citizen.

Mesh combines the things we actually cared about:

- Elixir/Ruby-style expressive syntax
- Static Hindley-Milner type inference
- BEAM-style concurrency and fault-tolerance
- LLVM-compiled native binaries
- **140% faster than Elixir** in our benchmarks

But raw speed isn't the headline. The headline is what happens when you treat distribution as a core language concept instead of a library you bolt on later.

In most languages, if you want distribution, failover, and load balancing, you go find a framework. You configure it. You learn its opinions. You debug the gap between what it promised and what it does under pressure.

In Mesh, those aren't add-ons. They're syntax.

Spinning up a distributed process, defining a failover strategy, or spreading load across nodes reads like the rest of your code — because it *is* the rest of your code. The distribution DX is the simplest we've seen, and that's not an accident. We designed Mesh around the reality that distributed systems aren't an edge case. They're the default.

In Hyperpush, every error event runs as its own lightweight process. Failures stay isolated. Supervision is part of the architecture, not an afterthought. The concurrency model fits the workload instead of fighting it.

Error tracking systems don't fail gracefully by default. Under stress, they drop events, become opaque, or turn into a debugging project of their own. We don't want Hyperpush to break exactly when you need it.

---

## Hyperpush is the product. Mesh is the reason it's built this way.

The point isn't "look, we made a language."

The point is that we wanted to build an error tracker we'd actually want to use — and the tools available kept forcing compromises we didn't want to make.

So here's what Hyperpush actually is:

**A real Sentry replacement** — open source, self-hostable, built for small teams.

**Deep GitHub integration** — when an error comes in, Hyperpush doesn't just log it. It traces it back through your commit history, identifies the likely root cause, and can open a fix as a PR. Automated root cause analysis. Automated remediation. Not a magic button that lies to you — a tight feedback loop between what broke and what changed.

**AI-native, not AI-sprinkled** — the entire operator experience can run through a chat agent if you want it to. You don't have to click through dashboards to understand what's on fire. You can just ask. Hyperpush's AI layer is deep enough that some operators may never touch the traditional UI at all.

The roadmap from here is straightforward:

1. A real Sentry replacement where the core product, standing on its own
2. A public bug board with clear issue visibility
3. Optional funding mechanics for open-source projects that want them

But the core has to stand on its own. If Hyperpush isn't a genuinely good error tracker without the extra layers, the rest is decoration.

---

## The bet

If a language can't power its own apps, it's not ready for yours.

Hyperpush isn't just an app built *with* Mesh. It's the clearest argument *for* Mesh.

We're building the product we wanted. We're building the language we wanted to build it in. And we think both get better because of that.

This is the start.

---