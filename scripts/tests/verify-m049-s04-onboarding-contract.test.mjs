import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')

const filePaths = {
  readme: 'README.md',
  scaffold: 'compiler/mesh-pkg/src/scaffold.rs',
  clusteredExample: 'website/docs/docs/getting-started/clustered-example/index.md',
  distributed: 'website/docs/docs/distributed/index.md',
  distributedProof: 'website/docs/docs/distributed-proof/index.md',
  tooling: 'website/docs/docs/tooling/index.md',
  clusteringSkill: 'tools/skill/mesh/skills/clustering/SKILL.md',
}

const retiredRepoRootDirs = ['tiny-cluster', 'cluster-proof']

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

function requireNoMatch(errors, relativePath, text, pattern, label) {
  if (pattern.test(text)) {
    errors.push(`${relativePath} still contains ${label}`)
  }
}

function extractClusteredScaffoldReadme(scaffoldSource) {
  const match = scaffoldSource.match(/let readme = format!\(\s*r#"([\s\S]*?)"#,\s*name = name\s*\);/)
  assert.ok(match, 'unable to locate clustered scaffold README template in compiler/mesh-pkg/src/scaffold.rs')
  return match[1]
}

function validateOnboardingContract(baseRoot) {
  const errors = []

  for (const relativePath of retiredRepoRootDirs) {
    if (fs.existsSync(path.join(baseRoot, relativePath))) {
      errors.push(`${relativePath} still exists as a repo-root proof package directory`)
    }
  }

  const readme = readFrom(baseRoot, filePaths.readme)
  const scaffoldSource = readFrom(baseRoot, filePaths.scaffold)
  const scaffoldReadme = extractClusteredScaffoldReadme(scaffoldSource)
  const clusteredExample = readFrom(baseRoot, filePaths.clusteredExample)
  const distributed = readFrom(baseRoot, filePaths.distributed)
  const distributedProof = readFrom(baseRoot, filePaths.distributedProof)
  const tooling = readFrom(baseRoot, filePaths.tooling)
  const clusteringSkill = readFrom(baseRoot, filePaths.clusteringSkill)

  requireIncludes(errors, filePaths.readme, readme, [
    'starter/examples-first',
    'examples/todo-postgres/README.md',
    'examples/todo-sqlite/README.md',
    'https://meshlang.dev/docs/production-backend-proof/',
    'mesher/README.md',
  ])

  requireIncludes(errors, `${filePaths.scaffold} clustered README template`, scaffoldReadme, [
    'examples/todo-postgres/README.md',
    'examples/todo-sqlite/README.md',
    'https://meshlang.dev/docs/production-backend-proof/',
    'mesher/README.md',
    'bash scripts/verify-m051-s01.sh',
    'bash scripts/verify-m051-s02.sh',
    'repo maintainers',
  ])

  requireIncludes(errors, filePaths.clusteredExample, clusteredExample, [
    'meshc init --clustered',
    'examples/todo-postgres/README.md',
    'examples/todo-sqlite/README.md',
    'After the scaffold, pick the follow-on starter',
    'Production Backend Proof',
  ])

  requireIncludes(errors, filePaths.distributed, distributed, [
    'public route scaffold/examples-first',
    'examples/todo-postgres/README.md',
    'examples/todo-sqlite/README.md',
    'Production Backend Proof',
    'mesher/README.md',
    'bash scripts/verify-m051-s01.sh',
    'bash scripts/verify-m051-s02.sh',
  ])

  requireIncludes(errors, filePaths.distributedProof, distributedProof, [
    'This is the only public-secondary docs page that carries the named clustered verifier rails.',
    'examples/todo-postgres/README.md',
    'examples/todo-sqlite/README.md',
    'Production Backend Proof',
    'mesher/README.md',
    'bash scripts/verify-m051-s01.sh',
    'bash scripts/verify-m051-s02.sh',
    'scripts/fixtures/clustered/tiny-cluster',
    'scripts/fixtures/clustered/cluster-proof',
    'public starter split plus a deeper backend handoff',
  ])

  requireIncludes(errors, filePaths.tooling, tooling, [
    'examples/todo-postgres/README.md',
    'examples/todo-sqlite/README.md',
    'Production Backend Proof',
    'starter/examples-first',
    'named backend proof surfaces instead of a first-contact repo-root runbook',
  ])

  requireIncludes(errors, filePaths.clusteringSkill, clusteringSkill, [
    'examples/todo-postgres',
    'examples/todo-sqlite',
    '/docs/production-backend-proof/',
    'mesher/README.md',
    'bash scripts/verify-m051-s01.sh',
    'bash scripts/verify-m051-s02.sh',
    'repo maintainers',
    '`meshc init --template todo-api --db postgres <name>` is the fuller shared or deployable starter layered on top of that same route-free clustered contract.',
    '`meshc init --template todo-api --db sqlite <name>` is the honest local single-node starter',
  ])

  for (const [relativePath, text] of [
    [filePaths.readme, readme],
    [`${filePaths.scaffold} clustered README template`, scaffoldReadme],
    [filePaths.clusteredExample, clusteredExample],
    [filePaths.distributed, distributed],
    [filePaths.distributedProof, distributedProof],
    [filePaths.tooling, tooling],
  ]) {
    requireNoMatch(errors, relativePath, text, /reference-backend\/README\.md/, 'repo-root reference-backend onboarding handoff')
    requireNoMatch(errors, relativePath, text, /tiny-cluster\/README\.md/, 'stale tiny-cluster onboarding link')
    requireNoMatch(errors, relativePath, text, /cluster-proof\/README\.md/, 'stale cluster-proof onboarding link')
    requireNoMatch(
      errors,
      relativePath,
      text,
      /meshc init --template todo-api(?! --db (sqlite|postgres))/,
      'unsplit todo-api starter guidance',
    )
  }

  requireNoMatch(
    errors,
    filePaths.clusteringSkill,
    clusteringSkill,
    /(keep|use) `reference-backend\/README\.md`|reference-backend\/README\.md` as the deeper backend proof/i,
    'repo-root reference-backend onboarding handoff',
  )
  requireNoMatch(errors, filePaths.clusteringSkill, clusteringSkill, /tiny-cluster\/README\.md/, 'stale tiny-cluster onboarding link')
  requireNoMatch(errors, filePaths.clusteringSkill, clusteringSkill, /cluster-proof\/README\.md/, 'stale cluster-proof onboarding link')
  requireNoMatch(
    errors,
    filePaths.clusteringSkill,
    clusteringSkill,
    /meshc init --template todo-api(?! --db (sqlite|postgres))/,
    'unsplit todo-api starter guidance',
  )

  requireNoMatch(
    errors,
    filePaths.distributedProof,
    distributedProof,
    /cargo run -q -p meshc -- build tiny-cluster\b/,
    'deleted root tiny-cluster build command',
  )
  requireNoMatch(
    errors,
    filePaths.distributedProof,
    distributedProof,
    /cargo run -q -p meshc -- test tiny-cluster\/tests\b/,
    'deleted root tiny-cluster test command',
  )
  requireNoMatch(
    errors,
    filePaths.distributedProof,
    distributedProof,
    /cargo run -q -p meshc -- build cluster-proof\b/,
    'deleted root cluster-proof build command',
  )
  requireNoMatch(
    errors,
    filePaths.distributedProof,
    distributedProof,
    /cargo run -q -p meshc -- test cluster-proof\/tests\b/,
    'deleted root cluster-proof test command',
  )

  return errors
}

test('current repo publishes the scaffold/examples-first clustered onboarding contract', () => {
  const errors = validateOnboardingContract(root)
  assert.deepEqual(errors, [], errors.join('\n'))
})

test('contract fails closed when README reintroduces a repo-root backend runbook as the public next step', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m049-s04-readme-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const relativePath = filePaths.readme
  let mutated = readFrom(tmpRoot, relativePath)
  mutated = mutated.replaceAll(
    'https://meshlang.dev/docs/production-backend-proof/',
    'https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md',
  )
  writeTo(tmpRoot, relativePath, mutated)

  const errors = validateOnboardingContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('README.md missing "https://meshlang.dev/docs/production-backend-proof/"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('README.md still contains repo-root reference-backend onboarding handoff')), errors.join('\n'))
})

test('contract fails closed when the clustered scaffold README drifts back toward a repo-root backend runbook', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m049-s04-scaffold-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const relativePath = filePaths.scaffold
  let mutated = readFrom(tmpRoot, relativePath)
  mutated = mutated.replaceAll(
    'https://meshlang.dev/docs/production-backend-proof/',
    'https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md',
  )
  mutated = mutated.replaceAll('mesher/README.md', 'reference-backend/README.md')
  writeTo(tmpRoot, relativePath, mutated)

  const errors = validateOnboardingContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('compiler/mesh-pkg/src/scaffold.rs clustered README template missing "https://meshlang.dev/docs/production-backend-proof/"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('compiler/mesh-pkg/src/scaffold.rs clustered README template missing "mesher/README.md"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('compiler/mesh-pkg/src/scaffold.rs clustered README template still contains repo-root reference-backend onboarding handoff')), errors.join('\n'))
})

test('contract fails closed when distributed proof drifts back to deleted repo-root fixture commands', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m049-s04-proof-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const relativePath = filePaths.distributedProof
  let mutated = readFrom(tmpRoot, relativePath)
  mutated = mutated.replaceAll('scripts/fixtures/clustered/tiny-cluster', 'tiny-cluster')
  mutated = mutated.replaceAll('scripts/fixtures/clustered/cluster-proof', 'cluster-proof')
  writeTo(tmpRoot, relativePath, mutated)

  const errors = validateOnboardingContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('website/docs/docs/distributed-proof/index.md missing "scripts/fixtures/clustered/tiny-cluster"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/distributed-proof/index.md missing "scripts/fixtures/clustered/cluster-proof"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/distributed-proof/index.md still contains deleted root tiny-cluster build command')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('website/docs/docs/distributed-proof/index.md still contains deleted root cluster-proof build command')), errors.join('\n'))
})

test('contract fails closed when the clustering skill collapses the deeper handoff back to repo-root backend teaching', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m049-s04-skill-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const relativePath = filePaths.clusteringSkill
  let mutated = readFrom(tmpRoot, relativePath)
  mutated = mutated.replaceAll('/docs/production-backend-proof/', 'reference-backend/README.md')
  mutated = mutated.replaceAll('mesher/README.md', 'reference-backend/README.md')
  mutated = mutated.replaceAll('meshc init --template todo-api --db postgres', 'meshc init --template todo-api')
  writeTo(tmpRoot, relativePath, mutated)

  const errors = validateOnboardingContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md missing "/docs/production-backend-proof/"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md missing "mesher/README.md"')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md still contains repo-root reference-backend onboarding handoff')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('tools/skill/mesh/skills/clustering/SKILL.md still contains unsplit todo-api starter guidance')), errors.join('\n'))
})

test('contract fails closed when retired proof-package directories reappear at repo root', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m049-s04-root-dirs-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  for (const relativePath of retiredRepoRootDirs) {
    writeTo(tmpRoot, path.join(relativePath, 'README.md'), `# ${relativePath}\n`)
  }

  const errors = validateOnboardingContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes('tiny-cluster still exists as a repo-root proof package directory')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('cluster-proof still exists as a repo-root proof package directory')), errors.join('\n'))
})
