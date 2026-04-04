import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')

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

function validateDocsContract(baseRoot) {
  const errors = []
  const readmePath = 'README.md'
  const toolingPath = 'website/docs/docs/tooling/index.md'
  const vscodeReadmePath = 'tools/editors/vscode-mesh/README.md'

  const readme = readFrom(baseRoot, readmePath)
  const tooling = readFrom(baseRoot, toolingPath)
  const vscodeReadme = readFrom(baseRoot, vscodeReadmePath)

  requireIncludes(errors, readmePath, readme, [
    'meshc update',
    'meshpkg update',
    '`main.mpl` remains the default executable entrypoint',
    'optional `[package].entrypoint = "lib/start.mpl"`',
  ])

  requireIncludes(errors, toolingPath, tooling, [
    '### Update an installed toolchain',
    'meshc update',
    'meshpkg update',
    '`main.mpl` stays the default executable entrypoint.',
    'project-root-relative `[package].entrypoint = "lib/start.mpl"`',
    'preserves project-root-relative `.mpl` paths',
    'lib/start.mpl',
    'main.mpl',
    'hidden paths',
    '*.test.mpl',
    'manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`',
    '`@cluster`, `@cluster(N)`, `#{...}`, and `${...}`',
    'bash scripts/verify-m048-s05.sh',
  ])

  requireIncludes(errors, vscodeReadmePath, vscodeReadme, [
    '`@cluster` / `@cluster(N)`',
    '`#{...}` plus `${...}`',
    'same-file go-to-definition',
    'manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`',
  ])

  requireExcludes(errors, vscodeReadmePath, vscodeReadme, [
    'jump to definitions across files',
  ])

  return errors
}

test('current repo publishes the retained S05 public truth markers', () => {
  const errors = validateDocsContract(root)
  assert.deepEqual(errors, [], errors.join('\n'))
})

test('contract fails closed when README loses update, entrypoint, or verifier markers', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m048-s05-readme-')
  for (const relativePath of [
    'README.md',
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const readmePath = 'README.md'
  let mutatedReadme = readFrom(tmpRoot, readmePath)
  mutatedReadme = mutatedReadme.replace('meshc update', 'meshc upgrade')
  mutatedReadme = mutatedReadme.replace('optional `[package].entrypoint = "lib/start.mpl"`', 'optional manifest override')
  writeTo(tmpRoot, readmePath, mutatedReadme)

  const errors = validateDocsContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('README.md missing "meshc update"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('README.md missing "optional `[package].entrypoint = \\"lib/start.mpl\\"`"')), errors.join('\n'))
})

test('contract fails closed when tooling docs lose update, publish, grammar, or verifier truth', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m048-s05-tooling-')
  for (const relativePath of [
    'README.md',
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const toolingPath = 'website/docs/docs/tooling/index.md'
  let mutatedTooling = readFrom(tmpRoot, toolingPath)
  mutatedTooling = mutatedTooling.replace('### Update an installed toolchain', '### Upgrade tools')
  mutatedTooling = mutatedTooling.replace('preserves project-root-relative `.mpl` paths', 'ships source files')
  mutatedTooling = mutatedTooling.replace('manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`', 'override-entry fixture')
  mutatedTooling = mutatedTooling.replace('`@cluster`, `@cluster(N)`, `#{...}`, and `${...}`', 'grammar parity')
  mutatedTooling = mutatedTooling.replace('bash scripts/verify-m048-s05.sh', 'bash scripts/verify-m048-s04.sh')
  writeTo(tmpRoot, toolingPath, mutatedTooling)

  const errors = validateDocsContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md missing "### Update an installed toolchain"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md missing "preserves project-root-relative `.mpl` paths"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md missing "manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md missing "`@cluster`, `@cluster(N)`, `#{...}`, and `${...}`"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/tooling/index.md missing "bash scripts/verify-m048-s05.sh"')), errors.join('\n'))
})

test('contract fails closed when the VS Code README reintroduces stale definition claims', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m048-s05-vscode-')
  for (const relativePath of [
    'README.md',
    'website/docs/docs/tooling/index.md',
    'tools/editors/vscode-mesh/README.md',
  ]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const readmePath = 'tools/editors/vscode-mesh/README.md'
  let mutatedReadme = readFrom(tmpRoot, readmePath)
  mutatedReadme = mutatedReadme.replaceAll('same-file go-to-definition', 'jump to definitions across files')
  mutatedReadme = mutatedReadme.replace('`@cluster` / `@cluster(N)`', 'decorators')
  mutatedReadme = mutatedReadme.replace('manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`', 'override-entry fixture')
  writeTo(tmpRoot, readmePath, mutatedReadme)

  const errors = validateDocsContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/editors/vscode-mesh/README.md still contains stale text "jump to definitions across files"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/editors/vscode-mesh/README.md missing "`@cluster` / `@cluster(N)`"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/editors/vscode-mesh/README.md missing "same-file go-to-definition"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/editors/vscode-mesh/README.md missing "manifest-first override-entry fixture rooted by `mesh.toml` + `lib/start.mpl`"')), errors.join('\n'))
})
