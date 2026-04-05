# Mesh Language

[![VS Code Marketplace Version](https://img.shields.io/visual-studio-marketplace/v/OpenWorthTechnologies.mesh-lang)](https://marketplace.visualstudio.com/items?itemName=OpenWorthTechnologies.mesh-lang)
[![VS Code Marketplace Installs](https://img.shields.io/visual-studio-marketplace/i/OpenWorthTechnologies.mesh-lang)](https://marketplace.visualstudio.com/items?itemName=OpenWorthTechnologies.mesh-lang)

Language support for [Mesh](https://meshlang.dev) -- an expressive, readable programming language with built-in concurrency via actors and supervision trees.

VS Code is a **first-class** editor host in the public Mesh tooling contract. The contract lives at [meshlang.dev/docs/tooling](https://meshlang.dev/docs/tooling/) and keeps this README scoped to the VS Code install, packaging, and run path.

## Features

- **Syntax Highlighting** -- shared TextMate grammar with verified scoping for keywords, `@cluster` / `@cluster(N)` decorators, types, literals, comments, module-qualified calls, and both `#{...}` plus `${...}` interpolation in double- and triple-quoted strings
- **Language Configuration** -- bracket matching, auto-closing pairs, and Mesh-specific indentation for `do`/`end` blocks
- **Verified LSP Diagnostics** -- real-time parse and type errors from the Mesh compiler
- **Verified Hover** -- inferred type information on hover
- **Verified Go to Definition** -- same-file go-to-definition inside backend-shaped project code
- **Verified Document Formatting** -- format the current Mesh document through `meshc lsp`
- **Verified Signature Help** -- parameter hints with active-parameter tracking for function calls

The current transport-level regression suite exercises the LSP path over real stdio JSON-RPC against a small backend-shaped Mesh project, so the documented editor experience stays tied to the same bounded tooling surface as the CLI. The editor-host smoke remains intentionally bounded to same-file go-to-definition inside backend-shaped project code plus clean diagnostics and hover for a manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`. The bundled syntax grammar is also verified through the shared VS Code/docs parity corpus, including `@cluster`, `@cluster(N)`, and both `#{...}` plus `${...}` inside double- and triple-quoted strings.

## Installation

Install Mesh first with the verified public installer pair `https://meshlang.dev/install.sh` and `https://meshlang.dev/install.ps1`. The public installers place both `meshc` and `meshpkg` on your PATH; the extension itself uses `meshc lsp`.

**macOS and Linux:**

```sh
curl -sSf https://meshlang.dev/install.sh | sh
```

**Windows x86_64 (PowerShell):**

```powershell
irm https://meshlang.dev/install.ps1 | iex
```

Verify the installed binaries:

```sh
meshc --version
meshpkg --version
```

For the broader backend/tooling proof behind this public install contract, start with [Production Backend Proof](https://meshlang.dev/docs/production-backend-proof/) and the public [Developer Tools](https://meshlang.dev/docs/tooling/) guide.

Then build the current packaged extension from source:

```sh
npm install
npm run compile
npm run package
```

The package step writes the current versioned artifact to `dist/mesh-lang-<version>.vsix`. To install that freshly built artifact into VS Code, run:

```sh
npm run install-local
```

## Verification

When you need the full repo-root public tooling/editor proof chain instead of only the extension-local package/install loop, run this from the repository root:

```bash
bash scripts/verify-m036-s03.sh
```

That verifier replays the docs contract, VitePress build, existing VSIX/public README proof, this real Extension Development Host smoke, and the repo-owned Neovim replay from one named-phase command.

## Requirements

The Mesh compiler (`meshc`) must be installed and available in your PATH. The verified public installers at `https://meshlang.dev/install.sh` and `https://meshlang.dev/install.ps1` install both `meshc` and `meshpkg`; this extension connects to the built-in language server provided by `meshc`.

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `mesh.lsp.path` | `meshc` | Path to the meshc binary. Must be in PATH, or provide an absolute path. |

## Release Notes

See [CHANGELOG.md](CHANGELOG.md) for a detailed list of changes in each release.
