import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')

const skillPaths = {
  root: 'tools/skill/mesh/SKILL.md',
  clustering: 'tools/skill/mesh/skills/clustering/SKILL.md',
  syntax: 'tools/skill/mesh/skills/syntax/SKILL.md',
  http: 'tools/skill/mesh/skills/http/SKILL.md',
}

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

function copySkillFiles(baseRoot) {
  for (const relativePath of Object.values(skillPaths)) {
    copyRepoFile(baseRoot, relativePath)
  }
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

function requireNoMatch(errors, relativePath, text, pattern, label) {
  if (pattern.test(text)) {
    errors.push(`${relativePath} still contains ${label}`)
  }
}

function validateSkillContract(baseRoot) {
  const errors = []
  const files = {
    [skillPaths.root]: readFrom(baseRoot, skillPaths.root),
    [skillPaths.clustering]: readFrom(baseRoot, skillPaths.clustering),
    [skillPaths.syntax]: readFrom(baseRoot, skillPaths.syntax),
    [skillPaths.http]: readFrom(baseRoot, skillPaths.http),
  }

  requireIncludes(errors, skillPaths.root, files[skillPaths.root], [
    'meshc init --clustered',
    'meshc init --template todo-api --db postgres',
    'meshc init --template todo-api --db sqlite',
    'honest local single-node starter',
    '`skills/clustering`',
    'meshc cluster status|continuity|diagnostics',
    'route users toward `meshc init --clustered` or `meshc init --template todo-api --db postgres` for clustered starters',
  ])

  requireIncludes(errors, skillPaths.clustering, files[skillPaths.clustering], [
    '`meshc init --clustered <name>` is the primary public clustered-app scaffold.',
    '`meshc init --template todo-api --db postgres <name>` is the fuller shared or deployable starter layered on top of that same route-free clustered contract.',
    'The PostgreSQL Todo starter keeps `work.mpl` on `@cluster pub fn sync_todos()`',
    'dogfoods explicit-count `HTTP.clustered(1, ...)` only on `GET /todos` and `GET /todos/:id`; `GET /health` plus mutating routes stay local.',
    '`meshc init --template todo-api --db sqlite <name>` is the honest local single-node starter',
    'The SQLite Todo starter does not claim `work.mpl`, `HTTP.clustered(...)`, `meshc cluster`, or clustered/operator proof surfaces.',
    'Use the Postgres Todo template when you need the packaged clustered HTTP starter',
    '/docs/production-backend-proof/',
    'mesher/README.md',
    'bash scripts/verify-m051-s01.sh',
    'bash scripts/verify-m051-s02.sh',
    'repo maintainers',
    'Do not teach `reference-backend/README.md` or fixture/runbook paths as the public next step',
    'meshc cluster status <node-name@host:port> --json',
    'meshc cluster continuity <node-name@host:port> --json',
    'meshc cluster continuity <node-name@host:port> <request-key> --json',
    'meshc cluster diagnostics <node-name@host:port> --json',
    'HTTP.clustered(handler)',
    'HTTP.clustered(1, handler)',
    'intentionally local-only and does not make `HTTP.clustered(...)` part of its public contract.',
  ])

  requireIncludes(errors, skillPaths.syntax, files[skillPaths.syntax], [
    '@cluster',
    '@cluster(N)',
    'Node.start_from_env()',
    'skills/clustering',
  ])

  requireIncludes(errors, skillPaths.http, files[skillPaths.http], [
    'HTTP.route(router, path, handler)',
    'HTTP.on_get',
    'HTTP.on_post',
    'HTTP.on_put',
    'HTTP.on_delete',
    'HTTP.clustered(handler)',
    'HTTP.clustered(1, handler)',
    'The shipped PostgreSQL Todo starter uses `HTTP.on_get`, `HTTP.on_post`, `HTTP.on_put`, and `HTTP.on_delete` while keeping clustered read routes bounded; the SQLite Todo starter keeps all routes local and does not claim `HTTP.clustered(...)`.',
    'Keep route-free `@cluster` declarations as the canonical clustered surface.',
    '`meshc init --template todo-api --db sqlite` is the honest local starter and does not make `HTTP.clustered(...)` part of its public contract.',
    '`GET /health` and mutating routes stay local in the shipped PostgreSQL Todo starter.',
    'skills/clustering',
  ])

  for (const [relativePath, text] of Object.entries(files)) {
    requireNoMatch(errors, relativePath, text, /\[cluster\]/, 'legacy manifest cluster stanza guidance')
    requireNoMatch(errors, relativePath, text, /clustered\(work\)/, 'legacy helper-shaped clustered guidance')
    requireNoMatch(errors, relativePath, text, /execute_declared_work/, 'stale helper-shaped work name')
    requireNoMatch(errors, relativePath, text, /Work\.execute_declared_work/, 'stale runtime helper name')
  }

  for (const relativePath of [skillPaths.root, skillPaths.clustering, skillPaths.http]) {
    requireNoMatch(
      errors,
      relativePath,
      files[relativePath],
      /meshc init --template todo-api(?! --db (sqlite|postgres))/,
      'unsplit todo-api starter guidance',
    )
  }

  requireNoMatch(
    errors,
    skillPaths.clustering,
    files[skillPaths.clustering],
    /(keep|use) `reference-backend\/README\.md`|reference-backend\/README\.md` as the deeper backend proof/i,
    'repo-root reference-backend onboarding handoff',
  )
  requireNoMatch(
    errors,
    skillPaths.clustering,
    files[skillPaths.clustering],
    /local SQLite-backed HTTP app/,
    'stale clustered SQLite starter wording',
  )
  requireNoMatch(
    errors,
    skillPaths.http,
    files[skillPaths.http],
    /The shipped Todo starter uses/,
    'generic clustered Todo wording',
  )

  return errors
}

test('current repo publishes the honest starter split across the Mesh skill bundle', () => {
  const errors = validateSkillContract(root)
  assert.deepEqual(errors, [], errors.join('\n'))
})

test('contract fails closed when the root skill collapses the starter split back to one generic todo-api command', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m048-s04-root-')
  copySkillFiles(tmpRoot)

  const rootPath = skillPaths.root
  let mutated = readFrom(tmpRoot, rootPath)
  mutated = mutated.replaceAll('meshc init --template todo-api --db postgres', 'meshc init --template todo-api')
  mutated = mutated.replaceAll('meshc init --template todo-api --db sqlite', 'meshc init --template todo-api')
  mutated = mutated.replace('honest local single-node starter', 'starter')
  writeTo(tmpRoot, rootPath, mutated)

  const errors = validateSkillContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/SKILL.md missing "meshc init --template todo-api --db postgres"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/SKILL.md missing "meshc init --template todo-api --db sqlite"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/SKILL.md still contains unsplit todo-api starter guidance')), errors.join('\n'))
})

test('contract fails closed when the clustering skill reintroduces the old repo-root backend handoff', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m048-s04-clustering-')
  copySkillFiles(tmpRoot)

  const clusteringPath = skillPaths.clustering
  let mutated = readFrom(tmpRoot, clusteringPath)
  mutated = mutated.replace(
    '`meshc init --template todo-api --db postgres <name>` is the fuller shared or deployable starter layered on top of that same route-free clustered contract.',
    '`meshc init --template todo-api <name>` is the fuller starter layered on top of that same contract.',
  )
  mutated = mutated.replace(
    '/docs/production-backend-proof/',
    'reference-backend/README.md',
  )
  mutated = mutated.replace(
    'mesher/README.md',
    'reference-backend/README.md',
  )
  mutated = mutated.replace(
    'Do not teach `reference-backend/README.md` or fixture/runbook paths as the public next step after the clustered scaffold or Todo starters.',
    'Use `reference-backend/README.md` as the deeper backend proof once the generated starters stop being enough.',
  )
  writeTo(tmpRoot, clusteringPath, mutated)

  const errors = validateSkillContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md missing "`meshc init --template todo-api --db postgres <name>` is the fuller shared or deployable starter layered on top of that same route-free clustered contract."')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md missing "Do not teach `reference-backend/README.md` or fixture/runbook paths as the public next step"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md still contains unsplit todo-api starter guidance')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md still contains repo-root reference-backend onboarding handoff')), errors.join('\n'))
})

test('contract fails closed when the HTTP skill extends clustered route wrappers back onto the SQLite starter', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m048-s04-http-')
  copySkillFiles(tmpRoot)

  const httpPath = skillPaths.http
  let mutated = readFrom(tmpRoot, httpPath)
  mutated = mutated.replace(
    'The shipped PostgreSQL Todo starter uses `HTTP.on_get`, `HTTP.on_post`, `HTTP.on_put`, and `HTTP.on_delete` while keeping clustered read routes bounded; the SQLite Todo starter keeps all routes local and does not claim `HTTP.clustered(...)`.',
    'The shipped Todo starter uses `HTTP.on_get`, `HTTP.on_post`, `HTTP.on_put`, and `HTTP.on_delete` while keeping clustered read routes bounded.',
  )
  mutated = mutated.replace(
    '`meshc init --template todo-api --db sqlite` is the honest local starter and does not make `HTTP.clustered(...)` part of its public contract.',
    '`meshc init --template todo-api` keeps clustered reads bounded in the starter.',
  )
  writeTo(tmpRoot, httpPath, mutated)

  const errors = validateSkillContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/http/SKILL.md missing "The shipped PostgreSQL Todo starter uses `HTTP.on_get`, `HTTP.on_post`, `HTTP.on_put`, and `HTTP.on_delete` while keeping clustered read routes bounded; the SQLite Todo starter keeps all routes local and does not claim `HTTP.clustered(...)`."')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/http/SKILL.md missing "`meshc init --template todo-api --db sqlite` is the honest local starter and does not make `HTTP.clustered(...)` part of its public contract."')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/http/SKILL.md still contains generic clustered Todo wording')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/http/SKILL.md still contains unsplit todo-api starter guidance')), errors.join('\n'))
})
