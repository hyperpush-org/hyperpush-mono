# mesh.nvim

Repo-owned Neovim support pack for Mesh.

Together with VS Code, Neovim is a **first-class** editor host in the public Mesh tooling contract: <https://meshlang.dev/docs/tooling/>.

This pack is installed through Neovim's native package runtime under `pack/*/start/mesh-nvim`, stays intentionally bounded to the audited classic syntax plus native `meshc lsp` path proven in this repository, and requires **Neovim 0.11+**.

## What this pack does

- Detects `*.mpl` as `filetype=mesh` through native runtime package loading.
- Applies classic Vim syntax highlighting for the audited Mesh interpolation/string plus `@cluster` / `@cluster(N)` decorator contract proven in this repository.
- Auto-enables a native `vim.lsp` config named `mesh` on Neovim 0.11+ without `nvim-lspconfig`.
- Starts `meshc lsp` with repo-local discovery that favors local dogfooding builds before falling back to well-known install paths or `PATH`.

## What this pack does **not** claim

- No Tree-sitter grammar.
- No plugin-manager-specific setup.
- No claims beyond the classic syntax plus native `meshc lsp` path proven in `scripts/verify-m036-s02.sh`.

## Install

Neovim only needs this directory to appear somewhere on `packpath` as `pack/*/start/mesh-nvim`.
A direct repo-local install looks like this:

```bash
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/nvim/site/pack/mesh/start"
ln -s \
  "/absolute/path/to/mesh-lang/tools/editors/neovim-mesh" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/nvim/site/pack/mesh/start/mesh-nvim"
```

Equivalent locations under any active `packpath` also work, as long as the final path shape is `pack/<group>/start/mesh-nvim`.

After installation, opening any `*.mpl` file should load:

- `ftdetect/mesh.vim` for `filetype=mesh`
- `syntax/mesh.vim` for classic syntax groups
- `plugin/mesh.lua` to auto-enable the native LSP config on supported Neovim versions

## `meshc` resolution and overrides

The LSP transport starts `meshc lsp` and resolves the binary in this order:

1. explicit override via `vim.g.mesh_lsp_path`
2. explicit override via `require('mesh').setup({ lsp_path = '/absolute/path/to/meshc' })`
3. repo/workspace-local `target/debug/meshc`
4. repo/workspace-local `target/release/meshc`
5. well-known install locations:
   - `~/.mesh/bin/meshc`
   - `/usr/local/bin/meshc`
   - `/opt/homebrew/bin/meshc`
6. `PATH`

Root detection is separate from binary discovery:

- workspace root prefers `mesh.toml`
- then falls back to root `main.mpl`
- then falls back to `.git`
- otherwise Mesh attaches in honest single-file mode (`root_dir = nil`)

If discovery fails, the pack reports the searched candidates and tells you to set one of the explicit overrides above.

## Verification

For the full repo-root public tooling/editor proof chain, run this from the repository root:

```bash
bash scripts/verify-m036-s03.sh
```

Use the Neovim-specific verifier below when you only need to replay this pack's bounded proof surface:

```bash
NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh
```

The command exits `0` only after these named phases pass:

1. `corpus` — materialize the shared S01 interpolation corpus, including markdown-backed docs snippets, into temporary `.mpl` files under `.tmp/m036-s02/`
2. `shared-grammar` — replay `bash scripts/verify-m036-s01.sh`
3. `upstream-lsp` — replay `cargo test -q -p meshc --test e2e_lsp -- --nocapture`
4. `neovim` — install this pack through a real `pack/*/start/mesh-nvim` path and run the headless Neovim smoke covering syntax plus LSP attach/root-resolution assertions for a backend-shaped manifest-rooted fixture, a manifest-first override-entry fixture, and honest single-file mode

The syntax side of phase 4 stays intentionally bounded: it replays the shared S01 interpolation corpus and then opens `scripts/fixtures/m048-s04-cluster-decorators.mpl` to prove `@cluster`, `@cluster(3)`, and bare `cluster` boundaries with explicit line/column synstack output.

The verifier emits phase-local logs and leaves artifacts under `.tmp/m036-s02/` so failures stay attributable by phase and case.

Optional narrow runs are available when you only need one side while iterating:

```bash
NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax
NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh lsp
NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh neovim
```
