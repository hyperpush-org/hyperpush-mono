"use client"

import { motion } from "framer-motion"
import { Check } from "lucide-react"
import { WaitlistButton } from "@/components/landing/waitlist-dialog"

const tiers = [
  {
    name: "Self-Hosted",
    price: "Free",
    description: "Run Mesher on your own infrastructure. Full error tracking, token economics, and the public bug board — no limits, no cost.",
    features: [
      "Unlimited events",
      "Mesh, JS/TS, Rust, Python & Node SDKs",
      "Project token launch",
      "Community bounties",
      "Alerts & stack traces",
      "Advanced alerts & integrations",
      "GitHub PR verification",
      "All AI features via Openrouter",
    ],
    cta: "View Docs",
    ctaHref: "/docs",
    popular: false,
  },
  {
    name: "Pro",
    price: "$29",
    period: "/month",
    description: "AI-powered analysis, private dashboards, and more volume for growing teams.",
    features: [
      "100K events/month",
      "30 AI root-cause analysis",
      "50 AI error grouping & fix suggestions",
      "Private + public dashboards",
      "Five team member",
      "One projects",
      "Priority support",
    ],
    cta: "Join Waitlist",
    popular: true,
    promo: "First 10 OSS projects get 6 months free",
    tokenUnlock: "$10K/month volume",
    tokenUnlockDetail: "Generate $10K/month in trading volume and Pro is free forever",
  },
  {
    name: "Pro+",
    price: "$100",
    period: "/month",
    description: "Everything in Pro with higher limits, extended retention, and team-scale volume.",
    features: [
      "1M events/month",
      "150 AI root-cause analysis",
      "300 AI error grouping & fix suggestions",
      "90-day data retention",
      "Unlimited team members",
      "Unlimited projects",
      "Dedicated support channel",
    ],
    cta: "Join Waitlist",
    popular: false,
    tokenUnlock: "$50K/month volume",
    tokenUnlockDetail: "Generate $50K/month in trading volume and Pro+ is free forever",
  },
]

export function Pricing() {
  return (
    <section id="pricing" className="relative py-20 sm:py-32 border-t border-border">
      <div className="max-w-7xl mx-auto px-4 sm:px-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className="text-center max-w-2xl mx-auto mb-12 sm:mb-16"
        >
          <p className="text-sm font-mono text-accent mb-3 sm:mb-4 uppercase tracking-wider">Pricing</p>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold tracking-tight mb-4 sm:mb-6 text-balance">
            Free to start.
            <br />
            <span className="text-muted-foreground">Scale when you need to.</span>
          </h2>
          <p className="text-base sm:text-lg text-muted-foreground text-pretty">
            Core error tracking, token economics, and the bug board are free forever. 
            Upgrade for AI-powered analysis and higher limits.
          </p>
          <p className="text-sm text-muted-foreground/70 mt-3 sm:mt-4">
            Or unlock paid tiers for free — your token earns 1% of every trade on bags.fm, offsetting your subscription automatically.
          </p>
        </motion.div>

        {/* Stacked on mobile, 3 cols on lg */}
        <div className="flex flex-col sm:grid sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-px sm:bg-border sm:rounded-xl sm:overflow-hidden">
          {tiers.map((tier, index) => (
            <motion.div
              key={tier.name}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: index * 0.1 }}
              className={`relative p-6 sm:p-8 rounded-xl sm:rounded-none border sm:border-0 ${
                tier.popular
                  ? "border-accent/30 bg-card sm:border-t-2 sm:border-t-accent"
                  : "border-border bg-background"
              }`}
            >
              {tier.popular && (
                <span className="absolute top-4 right-4 text-xs font-mono text-accent bg-accent/10 px-2 py-1 rounded">
                  Popular
                </span>
              )}
              
              <div className="mb-6 sm:mb-8">
                <h3 className="text-xl font-semibold mb-2">{tier.name}</h3>
                <div className="flex items-baseline gap-1 mb-3">
                  <span className="text-3xl sm:text-4xl font-bold">{tier.price}</span>
                  {tier.period && (
                    <span className="text-muted-foreground">{tier.period}</span>
                  )}
                </div>
                <p className="text-sm text-muted-foreground">{tier.description}</p>
                {tier.promo && (
                  <div className="mt-3 sm:mt-4 flex items-center gap-2 px-3 py-2 rounded-lg border border-accent/20 bg-accent/5">
                    <span className="text-accent text-xs">✦</span>
                    <span className="text-xs text-accent font-medium">{tier.promo}</span>
                  </div>
                )}
                {tier.tokenUnlock && (
                  <div className="mt-2 sm:mt-3 flex items-center gap-2 px-3 py-2 rounded-lg border border-border bg-muted/30">
                    <span className="text-xs">🪙</span>
                    <span className="text-xs text-muted-foreground">
                      <strong className="text-foreground">{tier.tokenUnlock}</strong> → free forever
                    </span>
                  </div>
                )}
              </div>

              <ul className="space-y-2.5 sm:space-y-3 mb-6 sm:mb-8">
                {tier.features.map((feature) => {
                  const isComingSoon = feature.endsWith(" [coming soon]")
                  const label = isComingSoon ? feature.slice(0, -" [coming soon]".length) : feature
                  return (
                    <li key={feature} className="flex items-start gap-3 text-sm">
                      <Check className={`w-4 h-4 mt-0.5 shrink-0 ${isComingSoon ? "text-muted-foreground/40" : "text-accent"}`} />
                      <span className={isComingSoon ? "text-muted-foreground/50" : "text-muted-foreground"}>
                        {label}
                        {isComingSoon && (
                          <span className="ml-2 text-[10px] font-mono uppercase tracking-wide text-muted-foreground/40 border border-border/60 px-1.5 py-0.5 rounded">
                            soon
                          </span>
                        )}
                      </span>
                    </li>
                  )
                })}
              </ul>

              {tier.ctaHref ? (
                <a
                  href={tier.ctaHref}
                  className="w-full inline-flex items-center justify-center rounded-md border border-border bg-transparent px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-accent/10 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                >
                  {tier.cta}
                </a>
              ) : (
                <WaitlistButton
                  className="w-full"
                  variant={tier.popular ? "default" : "outline"}
                >
                  {tier.cta}
                </WaitlistButton>
              )}
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

