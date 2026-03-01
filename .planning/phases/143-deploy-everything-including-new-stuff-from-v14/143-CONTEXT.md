# Phase 143: Deploy everything including new stuff from v14 - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Deploy all new v14.0 infrastructure and services to production: the package registry backend (Axum+PostgreSQL on Fly.io), the packages website (new SvelteKit app at packages.meshlang.dev), Cloudflare R2 tarball storage, meshpkg CLI binary distribution via updated installer, and trigger a fresh docs site deploy. Mesher is explicitly out of scope (it is a test/validation project, not a production service).

</domain>

<decisions>
## Implementation Decisions

### Registry infrastructure
- Platform: Fly.io (managed cloud)
- Database: Fly Postgres (managed Fly app, same Fly network as backend)
- Storage: Cloudflare R2 bucket — needs to be created fresh as part of this deployment (does not exist yet)
- Registry API domain: `api.packages.meshlang.dev` pointing to Fly.io deployment

### meshpkg binary distribution
- Distribution method: Bundle meshpkg into the existing meshc install script (`install.sh`) — one script installs both tools
- Target platforms: macOS arm64, macOS x86_64, Linux x86_64, Linux arm64 (same 4 targets as meshc)
- Release automation: GitHub Actions CI triggered on version tag push — extend the existing `release.yml` workflow which already handles all 4 targets for meshc
- No Windows builds for meshpkg (Windows not in the requested targets)

### Packages website
- Framework: SvelteKit (separate app, not VitePress)
- Domain: `packages.meshlang.dev`
- Purpose: Dynamic package browsing, search, and per-package pages with live data from the registry API
- Note: Phase 140 built VitePress package pages integrated into `meshlang.dev/packages/` — this phase ships a proper SvelteKit app at the dedicated subdomain instead
- Hosting platform for SvelteKit: Claude's Discretion (Vercel/Netlify/Cloudflare Pages all viable)

### Docs site
- Trigger a fresh GitHub Pages deploy via `deploy.yml` to ensure all v14 docs (phases 141–142 changes) are live on `meshlang.dev`
- No new content changes — this is a deployment confirmation step

### Deployment sequencing
- Order:
  1. Provision Fly.io app + Fly Postgres + Cloudflare R2 bucket + DNS entries
  2. Deploy registry backend to Fly.io (run migrations, verify API responds)
  3. Build and deploy SvelteKit packages site to static host
  4. Trigger GitHub Pages deploy for docs site
  5. Update `install.sh` to bundle meshpkg + extend `release.yml` for meshpkg binaries
  6. Tag a release to trigger CI and publish meshpkg + meshc binaries

### Mesher
- Explicitly excluded from this deployment phase — mesher is a test project that validates the Mesh language, not a production service

### Success criteria
- Services live + smoke test: registry API at `api.packages.meshlang.dev` responds, packages site at `packages.meshlang.dev` loads, `meshpkg publish` and `meshpkg install` work end-to-end against production

### Claude's Discretion
- SvelteKit hosting platform choice (Vercel, Netlify, or Cloudflare Pages)
- Specific Fly.io region selection for registry
- fly.toml configuration details
- R2 bucket naming
- SvelteKit app design/UI (fetch from registry API, no predefined design system required)

</decisions>

<specifics>
## Specific Ideas

- Extend existing `release.yml` (already builds meshc for 4 targets) rather than creating a new workflow
- The `.github/workflows/deploy.yml` can be manually triggered (`workflow_dispatch`) to force a fresh GitHub Pages deploy

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 143-deploy-everything-including-new-stuff-from-v14*
*Context gathered: 2026-03-01*
