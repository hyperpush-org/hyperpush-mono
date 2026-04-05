import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { execFileSync, spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')

function mkTmpDir(t, prefix) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix))
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }))
  return dir
}

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: options.cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, ...(options.env || {}) },
  })
}

function runAllowFailure(command, args, options = {}) {
  return spawnSync(command, args, {
    cwd: options.cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, ...(options.env || {}) },
  })
}

function setupRepo(t) {
  const repo = mkTmpDir(t, 'verify-whitespace-')
  fs.mkdirSync(path.join(repo, 'scripts'), { recursive: true })
  fs.copyFileSync(
    path.join(root, 'scripts', 'verify-whitespace.sh'),
    path.join(repo, 'scripts', 'verify-whitespace.sh'),
  )
  fs.chmodSync(path.join(repo, 'scripts', 'verify-whitespace.sh'), 0o755)

  run('git', ['init', '-q'], { cwd: repo })
  run('git', ['config', 'user.name', 'Mesh Test'], { cwd: repo })
  run('git', ['config', 'user.email', 'mesh-test@example.com'], { cwd: repo })
  return repo
}

test('staged mode trims trailing whitespace from fully staged text files', (t) => {
  const repo = setupRepo(t)
  const filePath = path.join(repo, 'example.ts')
  fs.writeFileSync(filePath, 'const answer = 42;   \nexport { answer };\t\n')
  run('git', ['add', 'example.ts'], { cwd: repo })

  const output = run('bash', ['scripts/verify-whitespace.sh', '--staged', '--fix'], { cwd: repo })
  assert.match(output, /staged diff is clean/)

  const rewritten = fs.readFileSync(filePath, 'utf8')
  assert.equal(rewritten, 'const answer = 42;\nexport { answer };\n')

  const stagedBlob = run('git', ['show', ':example.ts'], { cwd: repo })
  assert.equal(stagedBlob, rewritten)

  const diffCheck = runAllowFailure('git', ['diff', '--check', '--cached', '--'], { cwd: repo })
  assert.equal(diffCheck.status, 0, diffCheck.stderr || diffCheck.stdout)
})

test('staged fix refuses partially staged files instead of restaging whole files', (t) => {
  const repo = setupRepo(t)
  const filePath = path.join(repo, 'partial.ts')
  fs.writeFileSync(filePath, 'const first = 1;\n')
  run('git', ['add', 'partial.ts'], { cwd: repo })
  run('git', ['commit', '-qm', 'base'], { cwd: repo })

  fs.writeFileSync(filePath, 'const first = 1;   \n')
  run('git', ['add', 'partial.ts'], { cwd: repo })
  fs.writeFileSync(filePath, 'const first = 1;   \nconst second = 2;\n')

  const result = runAllowFailure('bash', ['scripts/verify-whitespace.sh', '--staged', '--fix'], { cwd: repo })
  assert.notEqual(result.status, 0)
  assert.match(result.stderr, /refusing to auto-fix partially staged files/)
})

test('diff-range mode fails closed when a committed diff introduces trailing whitespace', (t) => {
  const repo = setupRepo(t)
  const filePath = path.join(repo, 'range.ts')
  fs.writeFileSync(filePath, 'const clean = true;\n')
  run('git', ['add', 'range.ts'], { cwd: repo })
  run('git', ['commit', '-qm', 'base'], { cwd: repo })
  const base = run('git', ['rev-parse', 'HEAD'], { cwd: repo }).trim()

  fs.writeFileSync(filePath, 'const clean = true;   \n')
  run('git', ['add', 'range.ts'], { cwd: repo })
  run('git', ['commit', '-qm', 'introduce trailing whitespace'], { cwd: repo })

  const result = runAllowFailure('bash', ['scripts/verify-whitespace.sh', '--diff-range', `${base}..HEAD`], { cwd: repo })
  assert.notEqual(result.status, 0)
  assert.match(result.stderr, /committed diff range contains whitespace errors/)
  assert.match(result.stderr, /range\.ts:1:/)
})
