import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')
const m034HelperPath = path.join(root, 'scripts', 'lib', 'm034_public_surface_contract.py')

function readFrom(baseRoot, relativePath) {
  const absolutePath = path.join(baseRoot, relativePath)
  assert.ok(fs.existsSync(absolutePath), `missing ${relativePath}`)
  return fs.readFileSync(absolutePath, 'utf8')
}

function writeTo(baseRoot, relativePath, content) {
  const absolutePath = path.join(baseRoot, relativePath)
  fs.mkdirSync(path.dirname(absolutePath), { recursive: true })
  fs.writeFileSync(absolutePath, content)
}

function copyRepoFile(baseRoot, relativePath) {
  writeTo(baseRoot, relativePath, readFrom(root, relativePath))
}

function mkTmpDir(t, prefix) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix))
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }))
  return dir
}

function requireIncludes(errors, relativePath, text, needles) {
  for (const needle of needles) {
    if (!text.includes(needle)) {
      errors.push(`${relativePath} missing ${JSON.stringify(needle)}`)
    }
  }
}

function requireExcludes(errors, relativePath, text, needles) {
  for (const needle of needles) {
    if (text.includes(needle)) {
      errors.push(`${relativePath} still contains stale text ${JSON.stringify(needle)}`)
    }
  }
}

function requireMatches(errors, relativePath, text, checks) {
  for (const { regex, description } of checks) {
    if (!regex.test(text)) {
      errors.push(`${relativePath} missing ${description}`)
    }
  }
}

function validateSupportContract(baseRoot) {
  const errors = []
  const toolingPath = 'website/docs/docs/tooling/index.md'
  const vscodeReadmePath = 'tools/editors/vscode-mesh/README.md'
  const neovimReadmePath = 'tools/editors/neovim-mesh/README.md'

  const tooling = readFrom(baseRoot, toolingPath)
  const vscodeReadme = readFrom(baseRoot, vscodeReadmePath)
  const neovimReadme = readFrom(baseRoot, neovimReadmePath)

  requireIncludes(errors, toolingPath, tooling, [
    '### Support tiers',
    'Mesh only publishes repo-owned format-on-save guidance for the first-class editors in the [support tiers](#support-tiers) below.',
    'Best-effort editors should invoke `meshc fmt <file>` directly and treat that integration as user-maintained.',
    'The JSON-RPC transport is shared across editors, but Mesh only publishes repo-owned editor-host guidance for VS Code and Neovim.',
    'Best-effort editors that support LSP can point their client at:',
    'small backend-shaped Mesh project over real stdio JSON-RPC',
    'same-file go-to-definition inside backend-shaped project code',
    '### VS Code',
    'VS Code is a first-class editor host in the public Mesh tooling contract.',
    'bash scripts/verify-m036-s03.sh',
    '### Neovim',
    'Neovim is a first-class editor host in the public Mesh tooling contract for the audited classic syntax plus native `meshc lsp` path already proven in `scripts/verify-m036-s02.sh`.',
    'Use the Neovim-specific verifier below when you only need to replay this pack\'s bounded proof surface:',
    '### Best-effort editors',
    'Mesh does not publish repo-owned editor-host smoke, packaging, or troubleshooting guides for those setups.',
    '| VS Code Extension | -- | First-class VS Code editor host with verified Mesh LSP integration |',
    '| Neovim Pack | -- | First-class Neovim editor host for the classic syntax plus native `meshc lsp` path |',
  ])
  requireMatches(errors, toolingPath, tooling, [
    {
      regex: /\| First-class \| VS Code and Neovim \| Public docs, editor-specific READMEs, and repo-owned proof cover the published install\/run path\. \|/,
      description: 'the first-class support-tier table row',
    },
    {
      regex: /\| Best-effort \| Emacs, Helix, Zed, Sublime Text, TextMate reuse, and similar setups \| Reuse the shared `meshc lsp` transport or VS Code TextMate grammar, but Mesh does not publish repo-owned editor-host smoke for these integrations\. \|/,
      description: 'the best-effort support-tier table row',
    },
  ])
  requireExcludes(errors, toolingPath, tooling, [
    '### Other Editors',
    'Most editors can be configured to run the formatter automatically when you save a file.',
    'For other editors that support LSP (Neovim, Emacs, Helix, Zed), configure the language server command as:',
    'reference-backend/',
    'reference-backend/api/jobs.mpl',
    'scripts/fixtures/backend/reference-backend',
    '`reference-backend/README.md`',
  ])

  requireIncludes(errors, vscodeReadmePath, vscodeReadme, [
    'VS Code is a **first-class** editor host in the public Mesh tooling contract.',
    'https://meshlang.dev/docs/tooling/',
    'keeps this README scoped to the VS Code install, packaging, and run path.',
    'real stdio JSON-RPC against a small backend-shaped Mesh project',
    'same-file go-to-definition inside backend-shaped project code',
    'manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`',
    '## Verification',
    'bash scripts/verify-m036-s03.sh',
    'this real Extension Development Host smoke',
  ])
  requireExcludes(errors, vscodeReadmePath, vscodeReadme, [
    'reference-backend/',
    'reference-backend/api/jobs.mpl',
    'scripts/fixtures/backend/reference-backend',
    '`reference-backend/README.md`',
  ])

  requireIncludes(errors, neovimReadmePath, neovimReadme, [
    'Together with VS Code, Neovim is a **first-class** editor host in the public Mesh tooling contract:',
    'https://meshlang.dev/docs/tooling/',
    'stays intentionally bounded to the audited classic syntax plus native `meshc lsp` path proven in this repository',
    'No claims beyond the classic syntax plus native `meshc lsp` path proven in `scripts/verify-m036-s02.sh`.',
    'backend-shaped manifest-rooted fixture',
    'bash scripts/verify-m036-s03.sh',
    "Use the Neovim-specific verifier below when you only need to replay this pack's bounded proof surface:",
  ])
  requireExcludes(errors, neovimReadmePath, neovimReadme, [
    'reference-backend/',
    'scripts/fixtures/backend/reference-backend',
    'No public support-tier promise beyond the repo-local proof in `scripts/verify-m036-s02.sh`.',
    'No broader editor/tooling contract that belongs in later S03-facing docs.',
  ])

  return errors
}

function runM034Helper(args) {
  return spawnSync('python3', [m034HelperPath, ...args], {
    cwd: root,
    encoding: 'utf8',
  })
}

test('current repo publishes one explicit support-tier contract across tooling docs and editor READMEs', () => {
  const errors = validateSupportContract(root)
  assert.deepEqual(errors, [], errors.join('\n'))
})

test('contract validation fails closed when the support-tier heading disappears', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m036-s03-heading-')
  for (const relativePath of [
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
    'tools/editors/neovim-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const toolingPath = 'website/docs/docs/tooling/index.md'
  const mutatedTooling = readFrom(tmpRoot, toolingPath).replace('### Support tiers', '### Editor tiers')
  writeTo(tmpRoot, toolingPath, mutatedTooling)

  const errors = validateSupportContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md missing "### Support tiers"')), errors.join('\n'))
})

test('contract validation fails closed on stale Other Editors wording and best-effort drift', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m036-s03-other-editors-')
  for (const relativePath of [
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
    'tools/editors/neovim-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const toolingPath = 'website/docs/docs/tooling/index.md'
  let mutatedTooling = readFrom(tmpRoot, toolingPath)
  mutatedTooling = mutatedTooling.replace('### Best-effort editors', '### Other Editors')
  mutatedTooling = mutatedTooling.replace('| Best-effort | Emacs, Helix, Zed, Sublime Text, TextMate reuse, and similar setups | Reuse the shared `meshc lsp` transport or VS Code TextMate grammar, but Mesh does not publish repo-owned editor-host smoke for these integrations. |', '| Best-effort | Most editors | Reuse the shared `meshc lsp` transport. |')
  writeTo(tmpRoot, toolingPath, mutatedTooling)

  const errors = validateSupportContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md still contains stale text "### Other Editors"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md missing the best-effort support-tier table row')), errors.join('\n'))
})

test('contract validation fails closed when the tooling page reintroduces repo-root backend proof wording', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m036-s03-tooling-backend-')
  for (const relativePath of [
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
    'tools/editors/neovim-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const toolingPath = 'website/docs/docs/tooling/index.md'
  const mutatedTooling = readFrom(tmpRoot, toolingPath)
    .replace(
      'small backend-shaped Mesh project over real stdio JSON-RPC',
      '`reference-backend/` over real stdio JSON-RPC',
    )
    .replace(
      'same-file go-to-definition inside backend-shaped project code',
      'same-file go-to-definition on `reference-backend/api/jobs.mpl`',
    )
  writeTo(tmpRoot, toolingPath, mutatedTooling)

  const errors = validateSupportContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md still contains stale text "reference-backend/"') || error.includes('website/docs/docs/tooling/index.md still contains stale text "reference-backend/api/jobs.mpl"')), errors.join('\n'))
})

test('contract validation fails closed when the Neovim README reverts to withholding the public tier', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m036-s03-neovim-')
  for (const relativePath of [
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
    'tools/editors/neovim-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const neovimPath = 'tools/editors/neovim-mesh/README.md'
  const mutatedReadme = `${readFrom(tmpRoot, neovimPath)}\n- No public support-tier promise beyond the repo-local proof in \`scripts/verify-m036-s02.sh\`.\n`
  writeTo(tmpRoot, neovimPath, mutatedReadme)

  const errors = validateSupportContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/editors/neovim-mesh/README.md still contains stale text "No public support-tier promise beyond the repo-local proof in `scripts/verify-m036-s02.sh`."')), errors.join('\n'))
})

test('contract validation fails closed when the VS Code README reintroduces repo-root backend proof wording', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m036-s03-vscode-backend-')
  for (const relativePath of [
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
    'tools/editors/neovim-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const vscodePath = 'tools/editors/vscode-mesh/README.md'
  const mutatedReadme = readFrom(tmpRoot, vscodePath)
    .replace(
      'same-file go-to-definition inside backend-shaped project code',
      'same-file definition on `reference-backend/api/jobs.mpl`',
    )
  writeTo(tmpRoot, vscodePath, mutatedReadme)

  const errors = validateSupportContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/editors/vscode-mesh/README.md still contains stale text "reference-backend/"') || error.includes('tools/editors/vscode-mesh/README.md still contains stale text "reference-backend/api/jobs.mpl"')), errors.join('\n'))
})

test('contract validation fails closed when editor READMEs leak the retained fixture path', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m036-s03-retained-fixture-')
  for (const relativePath of [
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
    'tools/editors/neovim-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const neovimPath = 'tools/editors/neovim-mesh/README.md'
  const mutatedReadme = `${readFrom(tmpRoot, neovimPath)}\nMaintainer shortcut: open scripts/fixtures/backend/reference-backend directly when replaying this proof surface.\n`
  writeTo(tmpRoot, neovimPath, mutatedReadme)

  const errors = validateSupportContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/editors/neovim-mesh/README.md still contains stale text "scripts/fixtures/backend/reference-backend"')), errors.join('\n'))
})

test('the existing M034 tooling-page helper still passes on the current repo', () => {
  const result = runM034Helper(['local-docs', '--root', root])
  assert.equal(result.status, 0, result.stderr || result.stdout)
})

test('the existing M034 tooling-page helper still fails when a required runbook marker drifts', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m036-s03-m034-')
  for (const relativePath of [
    'README.md',
    'website/docs/docs/getting-started/index.md',
    'website/docs/docs/tooling/index.md',
    'website/docs/public/install.sh',
    'website/docs/public/install.ps1',
    'tools/editors/vscode-mesh/package.json',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const toolingPath = path.join(tmpRoot, 'website/docs/docs/tooling/index.md')
  const mutatedTooling = fs.readFileSync(toolingPath, 'utf8').replace('.tmp/m034-s05/verify/remote-runs.json', '')
  fs.writeFileSync(toolingPath, mutatedTooling)

  const result = runM034Helper(['local-docs', '--root', tmpRoot])
  assert.notEqual(result.status, 0, 'local-docs should fail when a required M034 tooling marker is missing')
  assert.match(result.stderr, /website\/docs\/docs\/tooling\/index\.md missing '.tmp\/m034-s05\/verify\/remote-runs\.json'/)
})
