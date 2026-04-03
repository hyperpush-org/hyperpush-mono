"use client"

import { Button } from "@/components/ui/button"
import { Github, AlertCircle, AlertTriangle, Info } from "lucide-react"
import { motion } from "framer-motion"
import { WaitlistButton } from "@/components/landing/waitlist-dialog"

const events = [
  {
    type: "TypeError",
    message: "Cannot read properties of undefined (reading 'map')",
    file: "src/components/Feed.tsx:47",
    env: "Chrome 121",
    severity: "high" as const,
    time: "just now",
    count: 14,
  },
  {
    type: "NetworkError",
    message: "Failed to fetch /api/v1/events — timeout after 10s",
    file: "src/lib/api.ts:112",
    env: "Firefox 123",
    severity: "medium" as const,
    time: "12s ago",
    count: 3,
  },
  {
    type: "ReferenceError",
    message: "analytics is not defined",
    file: "src/utils/tracking.ts:23",
    env: "Safari 17",
    severity: "low" as const,
    time: "1m ago",
    count: 1,
  },
]

const severityConfig = {
  high: {
    icon: AlertCircle,
    badge: "bg-destructive/10 text-destructive border-destructive/20",
    iconClass: "text-destructive",
  },
  medium: {
    icon: AlertTriangle,
    badge: "bg-amber-500/10 text-amber-400 border-amber-500/20",
    iconClass: "text-amber-400",
  },
  low: {
    icon: Info,
    badge: "bg-accent/10 text-accent border-accent/20",
    iconClass: "text-accent",
  },
}

// Simple sparkline path — errors over 30 min
const SPARKLINE = "M 0 42 C 15 38, 25 28, 40 32 S 60 24, 75 18 S 95 28, 110 14 S 130 8, 145 12 S 165 20, 180 10"

export function Hero() {
  return (
    <section className="relative flex items-start justify-center overflow-hidden sm:min-h-screen sm:items-center">
      {/* Grid background */}
      <div className="absolute inset-0 bg-[linear-gradient(to_right,rgba(255,255,255,0.02)_1px,transparent_1px),linear-gradient(to_bottom,rgba(255,255,255,0.02)_1px,transparent_1px)] bg-[size:64px_64px]" />

      {/* Gradient orb */}
      <div className="absolute top-1/2 left-1/2 h-[600px] w-[600px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-accent/5 blur-[120px] pointer-events-none sm:h-[800px] sm:w-[800px]" />

      <div className="relative z-10 mx-auto w-full max-w-5xl px-4 pt-8 pb-10 text-center sm:px-6 sm:pt-20 md:pt-24 sm:pb-24">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="mb-4 flex items-center justify-center gap-2 sm:mb-8"
        >
          <span className="inline-flex items-center gap-2 rounded-full border border-border bg-card/50 px-3 py-1.5 text-xs text-muted-foreground backdrop-blur-sm sm:px-4 sm:py-2 sm:text-sm">
            <span className="h-2 w-2 shrink-0 animate-pulse rounded-full bg-accent" />
            Built on Solana
          </span>
        </motion.div>

        <motion.h1
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="mx-auto mb-4 max-w-[11ch] text-4xl leading-[0.98] font-bold tracking-tight text-balance sm:mb-6 sm:max-w-none sm:text-5xl md:text-6xl lg:text-7xl"
        >
          Sentry, but it funds your project
          <br className="hidden sm:block" />
          <span className="sm:hidden"> </span>
          <span className="text-accent">instead of their VC</span>
        </motion.h1>

        <motion.p
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="mx-auto mb-6 max-w-[22rem] text-[15px] leading-7 text-muted-foreground text-pretty sm:hidden"
        >
          Drop-in replacement for Sentry — same SDK, better dashboards, zero lock-in. Ship a project token once and it
          funds your treasury while paying the devs who fix the bugs.
        </motion.p>

        <motion.p
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="hidden text-base text-muted-foreground text-pretty sm:mx-auto sm:mb-10 sm:block sm:max-w-3xl sm:text-lg md:text-xl"
        >
          Tired of writing checks to error-tracking companies? Hyperpush is the direct drop-in replacement for
          Sentry — same SDK, better dashboards, zero lock-in. The difference? Ship a project token once and it
          automatically fills your treasury while paying the devs who actually fix the bugs. Open-source error
          tracking that funds itself.
        </motion.p>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
          className="mb-8 flex flex-col items-center justify-center gap-3 sm:mb-16 sm:flex-row sm:gap-4"
        >
          <WaitlistButton size="lg" className="h-11 w-full gap-2 px-6 sm:h-12 sm:w-auto sm:px-8">
            Join Waitlist
          </WaitlistButton>
          <Button
            size="lg"
            variant="outline"
            className="h-11 w-full gap-2 px-6 sm:h-12 sm:w-auto sm:px-8"
            asChild
          >
            <a href="https://github.com/hyperpush-dev/hyperpush" target="_blank" rel="noopener noreferrer">
              <Github className="h-4 w-4" />
              View on GitHub
            </a>
          </Button>
        </motion.div>

        {/* Live error feed dashboard */}
        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 0.4 }}
          className="relative overflow-hidden rounded-2xl border border-border bg-card/80 text-left shadow-2xl backdrop-blur-sm"
        >
          {/* Header bar */}
          <div className="flex items-center justify-between border-b border-border bg-muted/30 px-3 py-2.5 sm:px-4 sm:py-3">
            <div className="flex items-center gap-2.5">
              <div className="h-2 w-2 animate-pulse rounded-full bg-destructive" />
              <span className="text-[11px] font-mono text-muted-foreground sm:text-xs">live error feed</span>
            </div>
            <div className="flex items-center gap-3 font-mono text-[11px] sm:gap-4 sm:text-xs">
              <span className="hidden text-muted-foreground/70 sm:inline">18 events/min</span>
              <span className="hidden h-3 w-px bg-border sm:block" />
              <span className="text-accent">99.8% uptime</span>
            </div>
          </div>

          {/* Error event rows */}
          <div className="divide-y divide-border/40">
            {events.map((event, i) => {
              const cfg = severityConfig[event.severity]
              const Icon = cfg.icon
              return (
                <motion.div
                  key={event.type + i}
                  initial={{ opacity: 0, x: -12 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.4, delay: 0.65 + i * 0.12 }}
                  className={`${i > 0 ? "hidden sm:flex" : "flex"} items-start gap-3 px-3 py-3 transition-colors hover:bg-muted/20 sm:px-4 sm:py-3.5`}
                >
                  <Icon className={`mt-0.5 h-4 w-4 shrink-0 ${cfg.iconClass}`} />
                  <div className="min-w-0 flex-1">
                    <div className="mb-0.5 flex flex-wrap items-center gap-2">
                      <span className="text-xs font-mono font-semibold text-foreground">{event.type}</span>
                      <span className={`rounded border px-1.5 py-0.5 text-[10px] font-mono ${cfg.badge}`}>
                        {event.severity}
                      </span>
                      {event.count > 1 && (
                        <span className="rounded bg-muted/60 px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground">
                          ×{event.count}
                        </span>
                      )}
                    </div>
                    <p className="truncate text-xs text-muted-foreground">{event.message}</p>
                    <p className="mt-0.5 hidden text-[10px] font-mono text-muted-foreground/50 sm:block">{event.file}</p>
                  </div>
                  <span className="hidden shrink-0 whitespace-nowrap pt-0.5 text-[10px] font-mono text-muted-foreground/40 sm:block">
                    {event.time}
                  </span>
                </motion.div>
              )
            })}
          </div>

          {/* Sparkline footer */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5, delay: 1.1 }}
            className="hidden items-end gap-4 border-t border-border bg-muted/20 px-4 py-3 sm:flex"
          >
            <div className="flex-1">
              <p className="mb-1.5 text-[10px] font-mono text-muted-foreground/40">errors — last 30 min</p>
              <svg viewBox="0 0 180 50" className="h-8 w-full" preserveAspectRatio="none">
                <defs>
                  <linearGradient id="sparkGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="rgb(89,193,132)" stopOpacity="0.25" />
                    <stop offset="100%" stopColor="rgb(89,193,132)" stopOpacity="0" />
                  </linearGradient>
                </defs>
                <path d={`${SPARKLINE} V 50 H 0 Z`} fill="url(#sparkGrad)" />
                <path
                  d={SPARKLINE}
                  fill="none"
                  stroke="rgb(89,193,132)"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </div>
            <div className="flex shrink-0 flex-col items-end gap-0.5">
              <span className="text-sm font-mono font-bold tabular-nums text-foreground">247</span>
              <span className="text-[10px] font-mono text-muted-foreground/40">total events</span>
            </div>
          </motion.div>
        </motion.div>
      </div>
    </section>
  )
}
