"use client"

import Link from "next/link"
import { motion } from "framer-motion"
import { ArrowLeft, ArrowRight, Zap, Shield, GitBranch, Layers, ChevronRight } from "lucide-react"
import { Header } from "@/components/landing/header"
import { Footer } from "@/components/landing/footer"
import { Button } from "@/components/ui/button"
import { MeshDataflow } from "@/components/landing/mesh-dataflow"

const pillars = [
  {
    icon: Shield,
    title: "Failures stay contained",
    tagline: "Actor isolation",
    description:
      "Every error event hyperpush receives runs as its own lightweight process — completely isolated from everything else. If one event causes a problem, it can't take down any others. The system keeps running as if nothing happened.",
    analogy:
      "Think of it like a building with fireproof walls between every room. A fire in one room doesn't spread.",
  },
  {
    icon: Zap,
    title: "140% faster than the alternatives",
    tagline: "Native compilation",
    description:
      "Mesh compiles down to native machine code via LLVM — the same technology powering Rust and C++. You get the speed of a systems language with the expressiveness of something far more pleasant to build in.",
    analogy:
      "Most error trackers are running an interpreted script. Hyperpush is running a compiled binary.",
  },
  {
    icon: GitBranch,
    title: "Distribution built into the language",
    tagline: "First-class clustering",
    description:
      "In most languages, making servers work together across a network is a library you bolt on. In Mesh, it's part of the language itself — the same syntax as the rest of your code. Failover, load balancing, clustering: all first-class.",
    analogy:
      "Instead of teaching a car to fly after the fact, Mesh was designed as an aircraft from the start.",
  },
  {
    icon: Layers,
    title: "Self-healing by default",
    tagline: "Supervision trees",
    description:
      "Every part of hyperpush has a supervisor watching it. When something crashes — and in distributed systems, things always crash eventually — the supervisor restarts it automatically, in the right state, with the right context.",
    analogy:
      "It's the difference between a city with ambulances and fire departments versus one that hopes nothing ever goes wrong.",
  },
]

const timeline = [
  {
    phase: "The problem",
    text: "Error tracking isn't simple. Events arrive in bursts, some need grouping, some need alerts, some need to fan out to automations — all without any single path being able to take the system down.",
  },
  {
    phase: "The existing options",
    text: "Fast languages made concurrency awkward. Distributed languages lacked the compiled, systems-level performance we needed. Most of them were unpleasant to build in for years at a stretch.",
  },
  {
    phase: "The decision",
    text: "We stopped trying to pick the least-wrong option. We built Mesh — combining Elixir's actor model and fault-tolerance story with LLVM-native speed and distribution as a first-class language feature.",
  },
  {
    phase: "The result",
    text: "hyperpush, built entirely in Mesh. Every event isolated. Every failure contained. Every recovery automatic. A system that gets more reliable under pressure, not less.",
  },
]

const stats = [
  { value: "140%", label: "faster than Elixir", sub: "in equivalent workloads" },
  { value: "<1ms", label: "process spawn time", sub: "for each error event" },
  { value: "∞", label: "concurrent processes", sub: "lightweight, isolated actors" },
  { value: "0", label: "shared state", sub: "between event handlers" },
]

export default function MeshPage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Header />

      {/* Hero */}
      <section className="relative overflow-hidden border-b border-border py-20 sm:py-28">
        {/* Grid */}
        <div className="absolute inset-0 bg-[linear-gradient(to_right,rgba(255,255,255,0.02)_1px,transparent_1px),linear-gradient(to_bottom,rgba(255,255,255,0.02)_1px,transparent_1px)] bg-[size:64px_64px]" />
        {/* Orb */}
        <div className="pointer-events-none absolute left-1/2 top-1/2 h-[500px] w-[500px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-accent/5 blur-[120px]" />

        <div className="relative z-10 mx-auto max-w-4xl px-4 sm:px-6">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
            className="mb-6"
          >
            <Link
              href="/"
              className="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
            >
              <ArrowLeft className="h-4 w-4" />
              Back to hyperpush
            </Link>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
          >
            <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-accent/20 bg-accent/5 px-3 py-1 text-xs font-mono uppercase tracking-wider text-accent">
              <span className="h-1.5 w-1.5 rounded-full bg-accent" />
              Built on Mesh
            </div>

            <h1 className="mb-6 max-w-3xl text-4xl font-bold tracking-tight text-balance sm:text-5xl md:text-6xl">
              We needed a better language.
              <br />
              <span className="text-muted-foreground">So we built one.</span>
            </h1>

            <p className="max-w-2xl text-lg text-muted-foreground text-pretty sm:text-xl">
              Mesh is the programming language hyperpush is written in. Not a dependency, not a framework —
              the actual language. We built it because nothing else gave us what we needed to build a
              reliable, high-throughput error tracker without compromise.
            </p>
          </motion.div>
        </div>
      </section>

      {/* What is Mesh — plain language */}
      <section className="py-16 sm:py-24">
        <div className="mx-auto max-w-4xl px-4 sm:px-6">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            <p className="mb-4 text-sm font-mono uppercase tracking-wider text-accent">What is Mesh?</p>
            <h2 className="mb-6 text-3xl font-bold tracking-tight sm:text-4xl">
              A programming language. The kind that runs the whole system.
            </h2>

            <div className="grid gap-8 sm:grid-cols-2">
              <div className="space-y-4 text-muted-foreground text-pretty">
                <p className="text-base leading-relaxed">
                  Software is written in programming languages. JavaScript runs websites. Python runs data
                  pipelines. Go runs backend services. Every piece of software you use — from your bank app
                  to the error tracker watching your code — was written in one.
                </p>
                <p className="text-base leading-relaxed">
                  Most developer tools are built with existing languages, designed for general use. Hyperpush
                  isn't. We wrote a new language, designed from scratch around the specific shape of problems
                  that error tracking at scale creates.
                </p>
              </div>
              <div className="space-y-4 text-muted-foreground text-pretty">
                <p className="text-base leading-relaxed">
                  Mesh takes its concurrency model from Erlang and Elixir — the languages behind WhatsApp,
                  Discord, and the most uptime-critical systems in the world — and combines it with the raw
                  compiled speed of native code.
                </p>
                <p className="text-base leading-relaxed">
                  The result: a system that handles millions of events, keeps failures isolated, heals
                  itself automatically, and runs faster than the tools it was inspired by. That's what
                  hyperpush runs on.
                </p>
              </div>
            </div>
          </motion.div>
        </div>
      </section>

      {/* ── Animated cluster visualization ───────────────────────────── */}
      <MeshDataflow />

      {/* Stats bar */}
      <section className="border-y border-border bg-card/30">
        <div className="mx-auto max-w-5xl px-4 sm:px-6">
          <div className="grid grid-cols-2 divide-x divide-y divide-border md:grid-cols-4 md:divide-y-0">
            {stats.map((stat, i) => (
              <motion.div
                key={stat.label}
                initial={{ opacity: 0, y: 12 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.4, delay: i * 0.08 }}
                className="flex flex-col items-center justify-center px-6 py-8 text-center"
              >
                <span className="mb-1 text-3xl font-bold tabular-nums text-accent sm:text-4xl">
                  {stat.value}
                </span>
                <span className="text-sm font-medium text-foreground">{stat.label}</span>
                <span className="mt-1 text-xs text-muted-foreground">{stat.sub}</span>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Why it was built — origin story */}
      <section className="py-16 sm:py-24">
        <div className="mx-auto max-w-4xl px-4 sm:px-6">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className="mb-12"
          >
            <p className="mb-4 text-sm font-mono uppercase tracking-wider text-accent">Why it exists</p>
            <h2 className="mb-6 max-w-2xl text-3xl font-bold tracking-tight sm:text-4xl text-balance">
              Error tracking is a concurrency problem in disguise.
            </h2>
            <p className="max-w-2xl text-lg text-muted-foreground text-pretty">
              It looks simple from the outside. It isn't. Here's the honest story of why we ended up
              building a language instead of just picking one off the shelf.
            </p>
          </motion.div>

          <div className="relative space-y-0">
            {/* Vertical line */}
            <div className="absolute left-[15px] top-4 hidden h-[calc(100%-2rem)] w-px bg-border sm:block" />

            {timeline.map((item, i) => (
              <motion.div
                key={item.phase}
                initial={{ opacity: 0, x: -16 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: i * 0.1 }}
                className="relative flex gap-6 pb-10 last:pb-0 sm:pl-10"
              >
                {/* Dot */}
                <div className="relative z-10 hidden h-8 w-8 shrink-0 items-center justify-center rounded-full border border-accent/30 bg-accent/10 text-xs font-mono font-bold text-accent sm:flex absolute left-0 top-0 -translate-x-1/2 -ml-[1px]">
                  {i + 1}
                </div>
                <div className="sm:pl-0">
                  <p className="mb-2 text-xs font-mono uppercase tracking-wider text-accent">{item.phase}</p>
                  <p className="text-base text-muted-foreground leading-relaxed text-pretty">{item.text}</p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Pullquote */}
      <section className="border-y border-border bg-accent/[0.03] py-14 sm:py-20">
        <div className="mx-auto max-w-3xl px-4 sm:px-6 text-center">
          <motion.blockquote
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            <p className="text-xl font-medium leading-relaxed text-balance sm:text-2xl md:text-3xl">
              "If a language can't power its own apps, it's not ready for yours. Hyperpush isn't just
              an app built{" "}
              <em className="not-italic text-accent">with</em> Mesh — it's the clearest argument{" "}
              <em className="not-italic text-accent">for</em> Mesh."
            </p>
            <footer className="mt-6 text-sm text-muted-foreground">— The hyperpush team</footer>
          </motion.blockquote>
        </div>
      </section>

      {/* How it works — 4 pillars */}
      <section className="py-16 sm:py-24">
        <div className="mx-auto max-w-5xl px-4 sm:px-6">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className="mb-12"
          >
            <p className="mb-4 text-sm font-mono uppercase tracking-wider text-accent">How it works</p>
            <h2 className="mb-4 max-w-2xl text-3xl font-bold tracking-tight sm:text-4xl text-balance">
              Four things that make hyperpush different at a systems level.
            </h2>
            <p className="max-w-xl text-lg text-muted-foreground text-pretty">
              You don't need to understand compilers to appreciate what this means for reliability.
            </p>
          </motion.div>

          <div className="grid gap-6 sm:grid-cols-2">
            {pillars.map((pillar, i) => (
              <motion.div
                key={pillar.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: i * 0.1 }}
                className="group rounded-2xl border border-border bg-card/50 p-6 transition-colors hover:border-accent/30 hover:bg-card/80 sm:p-8"
              >
                <div className="mb-5 flex items-start gap-4">
                  <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-border bg-muted transition-colors group-hover:border-accent/20 group-hover:bg-accent/10">
                    <pillar.icon className="h-5 w-5 text-accent" />
                  </div>
                  <div>
                    <p className="text-[11px] font-mono uppercase tracking-wider text-accent/70 mb-1">
                      {pillar.tagline}
                    </p>
                    <h3 className="text-lg font-semibold leading-snug sm:text-xl">{pillar.title}</h3>
                  </div>
                </div>
                <p className="mb-4 text-sm text-muted-foreground leading-relaxed text-pretty sm:text-base">
                  {pillar.description}
                </p>
                <div className="rounded-lg border border-accent/10 bg-accent/5 px-4 py-3">
                  <p className="text-xs text-accent/80 leading-relaxed text-pretty">
                    <span className="font-semibold">In plain terms: </span>
                    {pillar.analogy}
                  </p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Relationship — Mesh & hyperpush */}
      <section className="border-t border-border py-16 sm:py-24">
        <div className="mx-auto max-w-4xl px-4 sm:px-6">
          <div className="grid items-center gap-10 lg:grid-cols-2 lg:gap-16">
            <motion.div
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5 }}
            >
              <p className="mb-4 text-sm font-mono uppercase tracking-wider text-accent">
                Mesh & hyperpush
              </p>
              <h2 className="mb-6 text-3xl font-bold tracking-tight text-balance sm:text-4xl">
                Two products. One bet.
              </h2>
              <p className="mb-4 text-muted-foreground leading-relaxed text-pretty">
                Mesh is a real, general-purpose programming language. Other teams can use it to build
                other products. Hyperpush just happens to be the flagship proof that it works — an
                error tracker running at scale, in production, written entirely in the language itself.
              </p>
              <p className="text-muted-foreground leading-relaxed text-pretty">
                Every time hyperpush handles an error event reliably, every time the system recovers
                automatically from a failure, every time a new node joins the cluster without downtime —
                that's Mesh proving itself. Not in a benchmark. In the real product you're using.
              </p>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, x: 20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.15 }}
              className="space-y-3"
            >
              {[
                {
                  label: "hyperpush error event arrives",
                  detail: "Fan-out to grouping, alerting, and enrichment paths",
                  accent: false,
                },
                {
                  label: "Mesh spawns an isolated actor",
                  detail: "< 1ms. Completely independent from all other events.",
                  accent: true,
                },
                {
                  label: "Each path runs in parallel",
                  detail: "No shared state. No blocking. No bottlenecks.",
                  accent: false,
                },
                {
                  label: "Supervisor watches everything",
                  detail: "Any failure is caught and restarted automatically.",
                  accent: true,
                },
                {
                  label: "Event processed, UI updated",
                  detail: "You see the error. Nothing else was disrupted.",
                  accent: false,
                },
              ].map((step, i) => (
                <motion.div
                  key={step.label}
                  initial={{ opacity: 0, x: 12 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  viewport={{ once: true }}
                  transition={{ duration: 0.4, delay: 0.1 + i * 0.08 }}
                  className={`flex items-start gap-4 rounded-xl border p-4 transition-colors ${
                    step.accent
                      ? "border-accent/20 bg-accent/[0.04]"
                      : "border-border bg-card/40"
                  }`}
                >
                  <div
                    className={`mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[10px] font-mono font-bold ${
                      step.accent
                        ? "bg-accent/20 text-accent"
                        : "bg-muted text-muted-foreground"
                    }`}
                  >
                    {i + 1}
                  </div>
                  <div>
                    <p className="text-sm font-medium">{step.label}</p>
                    <p className="text-xs text-muted-foreground mt-0.5">{step.detail}</p>
                  </div>
                </motion.div>
              ))}
            </motion.div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="relative overflow-hidden border-t border-border py-20 sm:py-28">
        <div className="pointer-events-none absolute bottom-0 left-1/2 h-[300px] w-[500px] -translate-x-1/2 rounded-full bg-accent/8 blur-[100px]" />

        <div className="relative z-10 mx-auto max-w-3xl px-4 sm:px-6 text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            <h2 className="mb-4 text-3xl font-bold tracking-tight text-balance sm:text-4xl md:text-5xl">
              Ready to see what it can do?
            </h2>
            <p className="mx-auto mb-8 max-w-xl text-lg text-muted-foreground text-pretty">
              Hyperpush is the error tracker. Mesh is the engine. Together they're built for the
              workload that breaks other systems.
            </p>
            <div className="flex flex-col items-center justify-center gap-3 sm:flex-row sm:gap-4">
              <Button size="lg" className="h-12 w-full gap-2 px-8 sm:w-auto" asChild>
                <Link href="/">
                  Explore hyperpush
                  <ArrowRight className="h-4 w-4" />
                </Link>
              </Button>
              <Button size="lg" variant="outline" className="h-12 w-full gap-2 px-8 sm:w-auto" asChild>
                <a
                  href="https://meshlang.dev"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Visit meshlang.dev
                  <ChevronRight className="h-4 w-4" />
                </a>
              </Button>
            </div>
          </motion.div>
        </div>
      </section>

      <Footer />
    </div>
  )
}
