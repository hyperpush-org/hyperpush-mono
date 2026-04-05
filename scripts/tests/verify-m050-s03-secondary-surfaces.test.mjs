import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')

const files = {
  distributed: 'website/docs/docs/distributed/index.md',
  distributedProof: 'website/docs/docs/distributed-proof/index.md',
  productionBackendProof: 'website/docs/docs/production-backend-proof/index.md',
  web: 'website/docs/docs/web/index.md',
  databases: 'website/docs/docs/databases/index.md',
  testing: 'website/docs/docs/testing/index.md',
  concurrency: 'website/docs/docs/concurrency/index.md',
}

const allPaths = Object.values(files)
const genericGuideFiles = [files.web, files.databases, files.testing, files.concurrency]
const clusteredExampleLink = '[Clustered Example](/docs/getting-started/clustered-example/)'
const productionBackendProofLink = '[Production Backend Proof](/docs/production-backend-proof/)'
const mesherRunbookLink = '[`mesher/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md)'
const mesherVerifierCommand = 'bash scripts/verify-m051-s01.sh'
const retainedVerifierCommand = 'bash scripts/verify-m051-s02.sh'
const proofSurfaceVerifierCommand = 'bash scripts/verify-production-proof-surface.sh'
const staleRunbookLink = '[`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md)'
const staleFixturePath = 'scripts/fixtures/backend/reference-backend/'
const proofRoleSentence = "This is the compact public-secondary handoff for Mesh's backend proof story."
const proofAudienceSentence = 'Public readers should still stay scaffold/examples first:'
const proofBoundarySentence = 'This page only names the deeper maintainer surfaces behind that public story: Mesher as the maintained app, and a retained backend-only verifier kept behind a named replay instead of a public repo-root runbook.'
const distributedProofBoundarySentence = 'keep the deeper backend handoff on Production Backend Proof, Mesher, and the retained backend-only verifier instead of promoting any repo-root runbook as a coequal first-contact clustered starter'
const distributedProofStaleBoundarySentence = 'keep `reference-backend` as the deeper backend proof surface rather than a coequal first-contact clustered starter'
const distributedProofRoleSentence = 'This is the only public-secondary docs page that carries the named clustered verifier rails.'
const proofDirectRailMarkers = [
  'bash scripts/verify-m047-s04.sh',
  'bash scripts/verify-m047-s05.sh',
  'cargo test -p meshc --test e2e_m047_s07 -- --nocapture',
  'bash scripts/verify-m047-s06.sh',
  'bash scripts/verify-m043-s04-fly.sh --help',
]
const staleTestingExamples = [
  'meshc test reference-backend',
  'meshc test reference-backend/tests',
  'meshc test reference-backend/tests/config.test.mpl',
  'meshc test --coverage reference-backend',
]

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

function copyAllFiles(baseRoot) {
  for (const relativePath of allPaths) {
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

function requireExcludes(errors, relativePath, text, needles) {
  for (const needle of needles) {
    if (text.includes(needle)) {
      errors.push(`${relativePath} still contains stale text ${JSON.stringify(needle)}`)
    }
  }
}

function requireOrdered(errors, relativePath, text, labels) {
  let previousIndex = -1
  for (const label of labels) {
    const index = text.indexOf(label)
    if (index === -1) {
      errors.push(`${relativePath} missing ordered marker ${JSON.stringify(label)}`)
      return
    }
    if (index <= previousIndex) {
      errors.push(`${relativePath} drifted order around ${JSON.stringify(label)}`)
      return
    }
    previousIndex = index
  }
}

function validateSecondarySurfaces(baseRoot) {
  const errors = []
  const distributed = readFrom(baseRoot, files.distributed)
  const distributedProof = readFrom(baseRoot, files.distributedProof)
  const productionBackendProof = readFrom(baseRoot, files.productionBackendProof)

  requireIncludes(errors, files.distributed, distributed, [
    '> **Clustered proof surfaces:**',
    clusteredExampleLink,
    productionBackendProofLink,
    mesherRunbookLink,
    mesherVerifierCommand,
    retainedVerifierCommand,
  ])
  requireExcludes(errors, files.distributed, distributed, [
    staleRunbookLink,
    staleFixturePath,
    ...proofDirectRailMarkers,
  ])
  requireOrdered(errors, files.distributed, distributed, [
    clusteredExampleLink,
    productionBackendProofLink,
    mesherRunbookLink,
    mesherVerifierCommand,
    retainedVerifierCommand,
  ])

  requireIncludes(errors, files.distributedProof, distributedProof, [
    distributedProofRoleSentence,
    clusteredExampleLink,
    productionBackendProofLink,
    mesherRunbookLink,
    mesherVerifierCommand,
    retainedVerifierCommand,
    distributedProofBoundarySentence,
    '## Public surfaces and verifier rails',
    '## Named proof commands',
  ])
  requireExcludes(errors, files.distributedProof, distributedProof, [
    staleRunbookLink,
    staleFixturePath,
    distributedProofStaleBoundarySentence,
  ])
  requireOrdered(errors, files.distributedProof, distributedProof, [
    distributedProofRoleSentence,
    productionBackendProofLink,
    mesherRunbookLink,
    mesherVerifierCommand,
    retainedVerifierCommand,
  ])
  requireOrdered(errors, files.distributedProof, distributedProof, [
    distributedProofRoleSentence,
    '## Public surfaces and verifier rails',
    '## Named proof commands',
  ])

  requireIncludes(errors, files.productionBackendProof, productionBackendProof, [
    proofRoleSentence,
    proofAudienceSentence,
    proofBoundarySentence,
    '## Canonical surfaces',
    '## Named maintainer verifiers',
    '## Retained backend-only recovery signals',
    '## When to use this page vs the generic guides',
    '## Failure inspection map',
    clusteredExampleLink,
    mesherRunbookLink,
    mesherVerifierCommand,
    retainedVerifierCommand,
    proofSurfaceVerifierCommand,
    '[Web](/docs/web/)',
    '[Databases](/docs/databases/)',
    '[Testing](/docs/testing/)',
    '[Concurrency](/docs/concurrency/)',
    '[Developer Tools](/docs/tooling/)',
    'restart_count',
    'last_exit_reason',
    'recovered_jobs',
    'last_recovery_at',
    'last_recovery_job_id',
    'last_recovery_count',
    'recovery_active',
  ])
  requireExcludes(errors, files.productionBackendProof, productionBackendProof, [
    staleRunbookLink,
    staleFixturePath,
  ])
  requireOrdered(errors, files.productionBackendProof, productionBackendProof, [
    '## Canonical surfaces',
    '## Named maintainer verifiers',
    '## Retained backend-only recovery signals',
    '## When to use this page vs the generic guides',
    '## Failure inspection map',
  ])
  requireOrdered(errors, files.productionBackendProof, productionBackendProof, [
    clusteredExampleLink,
    mesherRunbookLink,
    mesherVerifierCommand,
    retainedVerifierCommand,
  ])

  for (const guidePath of genericGuideFiles) {
    const guide = readFrom(baseRoot, guidePath)
    requireIncludes(errors, guidePath, guide, [
      '> **Production backend proof:**',
      productionBackendProofLink,
      mesherRunbookLink,
      mesherVerifierCommand,
      retainedVerifierCommand,
    ])
    requireExcludes(errors, guidePath, guide, [
      staleRunbookLink,
      staleFixturePath,
    ])
    requireOrdered(errors, guidePath, guide, [
      productionBackendProofLink,
      mesherRunbookLink,
      mesherVerifierCommand,
      retainedVerifierCommand,
    ])
  }

  const testing = readFrom(baseRoot, files.testing)
  requireExcludes(errors, files.testing, testing, staleTestingExamples)

  return errors
}

test('current repo publishes the T02 secondary-surface Mesher and retained-proof contract', () => {
  const errors = validateSecondarySurfaces(root)
  assert.deepEqual(errors, [], errors.join('\n'))
})

test('contract fails closed when a generic guide skips Production Backend Proof or the Mesher/retained maintainer handoff', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s03-guides-')
  copyAllFiles(tmpRoot)

  let mutatedWeb = readFrom(tmpRoot, files.web)
  mutatedWeb = mutatedWeb.replace(productionBackendProofLink, '`reference-backend/README.md`')
  mutatedWeb = mutatedWeb.replace(mesherRunbookLink, staleRunbookLink)
  mutatedWeb = mutatedWeb.replace(retainedVerifierCommand, 'bash scripts/verify-old-backend.sh')
  writeTo(tmpRoot, files.web, mutatedWeb)

  const errors = validateSecondarySurfaces(tmpRoot)
  assert.ok(errors.some((error) => error.includes(`${files.web} missing ${JSON.stringify(productionBackendProofLink)}`) || error.includes(`${files.web} missing ordered marker ${JSON.stringify(productionBackendProofLink)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`${files.web} missing ${JSON.stringify(mesherRunbookLink)}`) || error.includes(`${files.web} still contains stale text ${JSON.stringify(staleRunbookLink)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`${files.web} missing ${JSON.stringify(retainedVerifierCommand)}`) || error.includes(`${files.web} drifted order around ${JSON.stringify(retainedVerifierCommand)}`)), errors.join('\n'))
})

test('contract fails closed when Production Backend Proof loses its compact role or leaks stale backend-only surfaces', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s03-proof-page-')
  copyAllFiles(tmpRoot)

  let mutatedProofPage = readFrom(tmpRoot, files.productionBackendProof)
  mutatedProofPage = mutatedProofPage.replace(proofRoleSentence, 'This is the backend docs page.')
  mutatedProofPage = mutatedProofPage.split(retainedVerifierCommand).join('bash scripts/verify-old-backend.sh')
  mutatedProofPage = `${mutatedProofPage}\n\nLeaked path: ${staleFixturePath}\n`
  writeTo(tmpRoot, files.productionBackendProof, mutatedProofPage)

  const errors = validateSecondarySurfaces(tmpRoot)
  assert.ok(errors.some((error) => error.includes(`${files.productionBackendProof} missing ${JSON.stringify(proofRoleSentence)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`${files.productionBackendProof} missing ${JSON.stringify(retainedVerifierCommand)}`) || error.includes(`${files.productionBackendProof} drifted order around ${JSON.stringify(retainedVerifierCommand)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`${files.productionBackendProof} still contains stale text ${JSON.stringify(staleFixturePath)}`)), errors.join('\n'))
})

test('contract fails closed when Distributed Proof or Testing reintroduce stale repo-root backend teaching', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s03-stale-backend-')
  copyAllFiles(tmpRoot)

  let mutatedDistributedProof = readFrom(tmpRoot, files.distributedProof)
  mutatedDistributedProof = mutatedDistributedProof.replace(mesherRunbookLink, staleRunbookLink)
  mutatedDistributedProof = mutatedDistributedProof.replace(
    distributedProofBoundarySentence,
    distributedProofStaleBoundarySentence,
  )
  writeTo(tmpRoot, files.distributedProof, mutatedDistributedProof)

  let mutatedTesting = readFrom(tmpRoot, files.testing)
  mutatedTesting = mutatedTesting.replace('meshc test my-app', 'meshc test reference-backend')
  mutatedTesting = mutatedTesting.replace('meshc test my-app/tests', 'meshc test reference-backend/tests')
  mutatedTesting = mutatedTesting.replace('meshc test my-app/tests/config.test.mpl', 'meshc test reference-backend/tests/config.test.mpl')
  mutatedTesting = mutatedTesting.replace('meshc test --coverage my-app', 'meshc test --coverage reference-backend')
  writeTo(tmpRoot, files.testing, mutatedTesting)

  const errors = validateSecondarySurfaces(tmpRoot)
  assert.ok(errors.some((error) => error.includes(`${files.distributedProof} missing ${JSON.stringify(mesherRunbookLink)}`) || error.includes(`${files.distributedProof} still contains stale text ${JSON.stringify(staleRunbookLink)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`${files.distributedProof} missing ${JSON.stringify(distributedProofBoundarySentence)}`) || error.includes(`${files.distributedProof} still contains stale text ${JSON.stringify(distributedProofStaleBoundarySentence)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`${files.testing} still contains stale text ${JSON.stringify('meshc test reference-backend')}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`${files.testing} still contains stale text ${JSON.stringify('meshc test --coverage reference-backend')}`)), errors.join('\n'))
})
