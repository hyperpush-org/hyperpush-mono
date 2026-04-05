# M056: Interactive Pitch Deck Page — Context

**Status:** Ready for planning

## What This Is

A dedicated `/pitch` page inside `mesher/landing/` (the hyperpush Next.js landing site) that presents an interactive, web-first pitch deck for hyperpush. It should feel native to the existing landing page's design language — dark theme, green accent, Geist fonts, framer-motion animations, canvas effects — while functioning as a self-contained slide-by-slide pitch experience in the browser, with PDF export capability.

## Stack & Design Constraints

The landing site is:
- **Next.js 16** (App Router, `mesher/landing/app/`)
- **Tailwind CSS v4** with oklch custom properties
- **framer-motion** for animations
- **Radix UI** component primitives via shadcn/ui
- **Geist / Geist Mono** fonts
- **Dark theme** with green accent (`oklch(0.75 0.18 160)` / `rgb(89, 193, 132)`)
- Deployed to Fly.io

The pitch deck page must:
- Live at `mesher/landing/app/pitch/page.tsx`
- Match the existing site's visual language (dark bg, green accent, grid patterns, gradient orbs, mono typography for labels)
- Be interactive and animated — scroll-driven or keyboard-navigable slide transitions
- Work as a web-first experience (watched in the browser)
- Support PDF export of slides
- Be responsive (desktop-first, but not broken on tablet/mobile)

## Content Sources

### Product context — hyperpush
- Landing page components in `mesher/landing/components/landing/` describe the product: open-source error tracking with Solana token economics, bug bounties, AI root-cause analysis
- `mesher/landing/lib/external-links.ts` has the GitHub, X, and Discord links
- The product positioning: "Sentry, but it funds your project instead of their VC"

### Language context — Mesh
- `website/docs/` contains the Mesh language documentation
- `.gsd/PROJECT.md` describes Mesh as a backend/distributed-systems language
- Key Mesh differentiators: native compilation, actor model, fault-tolerant clustering, `@cluster` decorator, runtime-owned failover, sub-millisecond process spawning
- The infrastructure section already positions Mesh as the backend: "Elixir's fault-tolerant actor model with raw compiled speed"

### Pitch narrative (suggested slide arc)
1. **Title** — hyperpush brand + tagline
2. **Problem** — error tracking is expensive, closed, and doesn't give back
3. **Solution** — open-source error tracking with built-in token economics
4. **How it works** — error flow → token → treasury → bounties → fixes
5. **Product** — interactive dashboard/feature showcase
6. **Infrastructure** — built on Mesh (the language story)
7. **Token economics** — the flywheel explained visually
8. **Traction / roadmap** — milestones, community, GitHub activity
9. **Team / open source** — community-driven, transparent
10. **CTA** — join waitlist, GitHub, Discord

## Acceptance Bar

- Page renders at `/pitch` in the existing Next.js app
- Slide navigation works via keyboard arrows, scroll, and clickable indicators
- Animations are smooth and match the landing page quality bar
- PDF export produces a reasonable multi-page document
- `npm --prefix mesher/landing run build` stays green
- No regressions to the existing landing page

## Out of Scope

- Changes to existing landing page components
- New backend/API work
- Real-time data integration
- Deployment (just build verification)
