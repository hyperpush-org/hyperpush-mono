# Contributing to Mesh

Thanks for contributing.

This repository contains the Mesh language, compiler/runtime/tooling crates, starter templates, examples, docs, and release verification scripts. Good changes here are usually small, explicit, and backed by the smallest truthful verification command.

## Before you start

- Search existing issues and pull requests first.
- For larger changes, behavior changes, or design changes, open an issue before writing a large PR.
- Keep changes scoped. Separate refactors from behavior changes unless they are inseparable.
- If you change public behavior, update the relevant docs, examples, starter templates, or release surfaces in the same PR.
- Follow the expectations in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
- For security issues, do **not** open a public issue. See [SECURITY.md](SECURITY.md).

## Development setup

### Required

- Rust toolchain
- Git

### Commonly needed

- Node.js and npm for `website/` and `packages-website/`
- Docker for starter, Postgres, and clustered/container verification flows

### Recommended repo hook setup

Install the repo-owned pre-commit hook once per clone:

```bash
bash scripts/install-git-hooks.sh
```

That hook runs `scripts/verify-whitespace.sh --staged --fix` before each commit. It trims safe staged trailing whitespace automatically, then fails closed if whitespace errors remain.

Git cannot force local hooks from a clone, so GitHub enforcement also lives in CI. Pull requests and `main` pushes run the same whitespace guard on the incoming diff.

## Common commands

Use the lightest command that truthfully proves your change.

### Rust workspace

```bash
cargo build
cargo test -p <crate>
```

Examples:

```bash
cargo test -p meshc -- --nocapture
cargo test -p mesh-lsp -- --nocapture
cargo test -p mesh-rt -- --nocapture
```

### Docs site

```bash
npm --prefix website ci
npm --prefix website run build
```

### Packages website

```bash
npm --prefix packages-website ci
npm --prefix packages-website run build
```

### Repo-owned verification scripts

This repo also carries many targeted verification rails under `scripts/verify-*.sh` and `scripts/verify-*.ps1`.

If your change touches a retained proof surface, starter template, editor integration, or release workflow, rerun the relevant verifier instead of relying only on broad workspace tests.

## Pull request guidelines

A good PR usually includes:

- a clear summary of what changed and why
- a linked issue when the change is non-trivial
- the exact verification commands that were run
- docs/example/template updates when public behavior changed
- focused diffs without unrelated churn

Use the pull request template and paste the exact commands you ran.

## Code and docs expectations

- Follow the surrounding style of the crate, package, or docs area you are editing.
- Prefer targeted tests over broad incidental rewrites.
- Keep starter and docs workflows honest: do not document a command path that the repo does not actually verify.
- Do not commit secrets, local `.env` files, generated release artifacts, or transient `.tmp/` output.

## Where to file what

- Reproducible defects: use the **Bug report** issue form.
- New capabilities or workflow improvements: use the **Feature request** issue form.
- Docs problems: use the **Documentation issue** form.
- Security reports: use the private path described in [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions will be licensed under the project license in [LICENSE](LICENSE).
