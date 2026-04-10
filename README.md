# Hyperpush Mono

This repo is the product-only root for Hyperpush in the blessed two-repo workspace.

It owns:

- `mesher/` — the Mesher app/runtime package
- `mesher/landing/` — the landing site
- `mesher/frontend-exp/` — the product-owned frontend experiment/dashboard surface
- product-root CI, docs, and maintainer verifiers that belong with those surfaces

It does **not** own the Mesh language/compiler/runtime/docs/registry/packages-site tree. That stays in the sibling `mesh-lang` repo.

## Blessed sibling workspace

```text
<workspace>/
  mesh-lang/
  hyperpush-mono/
    mesher/
    mesher/landing/
    mesher/frontend-exp/
```

The blessed product package root remains `hyperpush-mono/mesher/...`.
Do not flatten the product package to `<workspace>/mesher`.

## Repo-root maintainer surfaces

- `mesher/README.md` — Mesher maintainer runbook
- `bash mesher/scripts/verify-maintainer-surface.sh` — package-owned Mesher maintainer replay
- `bash scripts/verify-landing-surface.sh` — landing/root-surface verifier
- `.github/workflows/ci.yml` — product CI for Mesher + landing + `frontend-exp`
- `.github/workflows/deploy-landing.yml` — landing deploy/build workflow
- `.github/dependabot.yml` — product-owned dependency update scope

## Toolchain boundary

Mesher build/test/migrate/smoke flows need `meshc`.

The supported paths are:

1. a blessed sibling `mesh-lang/` checkout with `target/debug/meshc`
2. an explicit `MESHER_MESHC_BIN` + `MESHER_MESHC_SOURCE` override
3. `meshc` on `PATH`

If you are working in the blessed sibling workspace, the normal path is:

```text
<workspace>/
  mesh-lang/
  hyperpush-mono/
```

## Product repo identity

Canonical product repo URL: https://github.com/hyperpush-org/hyperpush

Local split-workspace paths may still use `hyperpush-mono/` as a compatibility checkout name.

The landing app and `frontend-exp` stay product-owned here.

## Working rules

- product changes live here
- language/toolchain changes live in `mesh-lang`
- if a product workflow depends on Mesh tooling, wire it to the sibling `mesh-lang` checkout or an explicit `meshc` override instead of copying compiler sources into this repo

## Git safety in the split workspace

If you are working in the blessed sibling workspace, install the tracked split-workspace hooks once from `mesh-lang/`:

```bash
bash ../mesh-lang/scripts/workspace-git.sh install-hooks
```

Then check both repos before committing or pushing:

```bash
bash ../mesh-lang/scripts/workspace-git.sh status
```

For a standalone `hyperpush-mono` clone with no sibling `mesh-lang` checkout, use the repo-local installer instead:

```bash
bash scripts/install-git-hooks.sh
```

The product repo `pre-push` hook blocks accidental partial pushes whenever the sibling `mesh-lang` repo is present and still dirty.
In a standalone product clone, the same tracked hook stays active but skips the cross-repo dirty-check because there is no sibling repo to inspect.
If you intentionally need a one-sided push, override the guard for that command only with `M055_ALLOW_PARTIAL_PUSH=1 git push ...`.
