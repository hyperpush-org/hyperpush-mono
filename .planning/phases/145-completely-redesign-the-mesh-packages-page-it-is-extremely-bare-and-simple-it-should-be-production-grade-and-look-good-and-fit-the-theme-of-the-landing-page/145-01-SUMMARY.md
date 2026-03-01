---
phase: 145-packages-redesign
plan: "01"
subsystem: packages-website, registry
tags: [tailwind, design-system, api-fix, svelte]
dependency_graph:
  requires: []
  provides: [tailwind-v4-foundation, oklch-token-system, versions-api-endpoint]
  affects: [packages-website, registry]
tech_stack:
  added: [tailwindcss@4, "@tailwindcss/vite", "@tailwindcss/typography", lucide-svelte, marked]
  patterns: [tailwind-v4-css-config, oklch-design-tokens, axum-route-handler]
key_files:
  created:
    - packages-website/src/app.css
  modified:
    - packages-website/vite.config.js
    - packages-website/package.json
    - packages-website/src/app.html
    - packages-website/src/routes/search/+page.server.js
    - registry/src/routes/metadata.rs
    - registry/src/routes/mod.rs
decisions:
  - tailwindcss() placed before sveltekit() in vite plugins — reversed order causes CSS classes to have no effect
  - versions route registered before /{name}/{version} in mod.rs — prevents Axum matching "versions" literal as a version param
  - search loader keeps ?q= in URL (user-facing) but sends ?search= to registry API (backend contract)
  - Tailwind v4 uses no tailwind.config.js — configuration is entirely CSS-based via @theme inline
metrics:
  duration: "~2 minutes"
  completed: "2026-03-01T23:05:44Z"
  tasks_completed: 3
  files_modified: 7
---

# Phase 145 Plan 01: Tailwind v4 Foundation + API Bug Fixes Summary

**One-liner:** Tailwind v4 with OKLCH token design system installed into packages-website; three API data bugs fixed and missing registry versions endpoint added.

## What Was Built

### Tailwind v4 Design System Foundation

`packages-website/src/app.css` is the single Tailwind v4 entry point. It contains:

- `@import "tailwindcss"` and `@plugin "@tailwindcss/typography"` at the top
- Inter and JetBrains Mono `@font-face` declarations (Latin-ext + Latin unicode ranges) using exact Google Fonts CDN URLs
- Full OKLCH token set in `:root` and `.dark` blocks copied verbatim from the landing page's `website/docs/.vitepress/theme/styles/main.css`
- `@theme inline` block mapping all CSS vars to Tailwind color/font utility names
- `@layer base` setting border, font smoothing, and body bg/text defaults
- Reveal animations: `.reveal`, `.reveal.is-visible`, `.reveal-delay-1/2/3/4`

`packages-website/vite.config.js` was updated to import and register `@tailwindcss/vite` as `tailwindcss()` before `sveltekit()` in the plugins array.

`packages-website/src/app.html` received an inline dark-mode FOUC prevention script in `<head>` before `%sveltekit.head%`.

### API Bug Fixes

**Bug 1 — Search query param mismatch (FIXED):** The search loader was sending `?q=` to the registry API, but the registry's `SearchParams` struct only reads `params.search`. Fixed to send `?search=` while the user-facing URL still uses `?q=` for clean URLs.

**Bug 2 — Missing versions HTTP endpoint (ADDED):** `db::packages::list_versions()` existed in the DB layer but had no HTTP route. Added `versions_handler` to `registry/src/routes/metadata.rs` and registered it at `GET /api/v1/packages/{name}/versions` in `mod.rs`, placed before `/{name}/{version}` to prevent Axum matching the literal "versions" as a version parameter.

## Commits

| Task | Description | Hash |
|------|-------------|------|
| 1 | Install Tailwind v4 deps, update vite.config.js | 37574d8b |
| 2 | Create app.css OKLCH design system, update app.html | bb957b3d |
| 3 | Fix search API bug, add registry versions endpoint | bef33cc1 |

## Verification

- `npm run build` in packages-website: SUCCESS (Tailwind v4 plugin processes app.css)
- `cargo check` in registry: PASSES (no errors, pre-existing dead code warnings only)
- `app.css` contains `@import "tailwindcss"`, 18 `oklch(` values, 6 `reveal` references
- `app.html` contains `prefers-color-scheme` dark mode detection
- Search loader sends `?search=` to registry API

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

Files created/modified verified:
- packages-website/src/app.css — FOUND
- packages-website/vite.config.js — FOUND (tailwindcss() before sveltekit())
- packages-website/src/app.html — FOUND (dark mode script present)
- packages-website/src/routes/search/+page.server.js — FOUND (?search= in URL)
- registry/src/routes/metadata.rs — FOUND (versions_handler present)
- registry/src/routes/mod.rs — FOUND (versions route before version route)

Commits verified:
- 37574d8b — FOUND
- bb957b3d — FOUND
- bef33cc1 — FOUND
