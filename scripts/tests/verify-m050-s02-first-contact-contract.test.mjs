import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')

const readmePath = 'README.md'
const gettingStartedPath = 'website/docs/docs/getting-started/index.md'
const clusteredExamplePath = 'website/docs/docs/getting-started/clustered-example/index.md'
const toolingPath = 'website/docs/docs/tooling/index.md'
const starterCommands = [
  'meshc init --clustered',
  'meshc init --template todo-api --db sqlite',
  'meshc init --template todo-api --db postgres',
]
const currentRepoUrl = 'https://github.com/snowdamiz/mesh-lang.git'
const staleRepoUrl = 'https://github.com/hyperpush-org/hyperpush-mono.git'
const currentRepoBlobBase = 'https://github.com/snowdamiz/mesh-lang/blob/main/'
const staleRepoBlobBase = 'https://github.com/hyperpush-org/hyperpush-mono/blob/main/'
const sqliteStarterLink = `${currentRepoBlobBase}examples/todo-sqlite/README.md`
const postgresStarterLink = `${currentRepoBlobBase}examples/todo-postgres/README.md`
const readmeLadderIntro = 'Keep the public ladder starter/examples-first: the scaffold and `/examples` stay ahead of maintainer proof surfaces.'
const readmeClusteredNextStep = '- **Clustered walkthrough:** use `meshc init --clustered` and then follow https://meshlang.dev/docs/getting-started/clustered-example/'
const readmeSqliteNextStep = `- **SQLite Todo starter:** ${sqliteStarterLink}`
const readmePostgresNextStep = `- **PostgreSQL Todo starter:** ${postgresStarterLink}`
const readmeProofNextStep = '- **Production Backend Proof:** https://meshlang.dev/docs/production-backend-proof/ — only after the starter/examples-first ladder when you need the maintainer-facing deeper backend proof page.'
const readmeToolingNextStep = '- **Tooling docs:** https://meshlang.dev/docs/tooling/'
const readmeMaintainerHeading = '## Maintainers / public release proof'
const readmeMaintainerMarker = '`mesher/README.md`'
const readmeMaintainerVerifierMarker = 'named maintainer verifier commands surfaced from that proof page'
const gettingStartedStarterHeading = '## Choose your next starter'
const gettingStartedLadderIntro = 'Keep the public first-contact ladder explicit and ordered: clustered scaffold first, then the honest local SQLite starter, then the serious shared/deployable PostgreSQL starter, and only then the maintainer-facing backend proof page.'
const gettingStartedClusteredNextStep = '- [Clustered Example](/docs/getting-started/clustered-example/)'
const gettingStartedSqliteNextStep = `- [SQLite Todo starter](${sqliteStarterLink})`
const gettingStartedPostgresNextStep = `- [PostgreSQL Todo starter](${postgresStarterLink})`
const gettingStartedProofNextStep = '- [Production Backend Proof](/docs/production-backend-proof/) -- the maintainer-facing backend proof page after the starter/examples-first ladder'
const clusteredExampleIntroMarker = 'This page stays on that scaffold first. Once you have the route-free clustered contract in hand, keep the public follow-on ladder ordered: honest local SQLite starter, serious shared/deployable PostgreSQL starter, then Production Backend Proof only when you need the maintainer-facing deeper backend proof. The retained verifier map stays behind the proof pages instead of a direct repo-root runbook handoff.'
const clusteredExampleStarterHeading = '## After the scaffold, pick the follow-on starter'
const clusteredExampleLadderIntro = 'Take the public follow-on ladder in order: honest local SQLite starter, serious shared/deployable PostgreSQL starter, then Production Backend Proof only when you need the maintainer-facing deeper backend proof.'
const clusteredExampleProofHeading = '## Need the retained verifier map?'
const clusteredExampleProofPage = '/docs/distributed-proof/'
const clusteredExampleProofNextStep = '- [Production Backend Proof](/docs/production-backend-proof/) — the maintainer-facing backend proof page after the starter/examples-first ladder.'
const toolingWorkflowMarker = 'Keep the public CLI workflow explicit and examples-first: hello world first, then the clustered scaffold, then the honest local SQLite starter or the serious shared/deployable PostgreSQL starter, and only after that the maintainer-facing backend proof page.'
const toolingStarterLadderIntro = 'After that CLI order, keep the public follow-on ladder explicit:'
const toolingProofNextStep = '- [Production Backend Proof](/docs/production-backend-proof/) — the maintainer-facing backend proof page after the starter/examples-first ladder'
const toolingDocsVerifierHeading = '## Assembled first-contact docs verifier'
const toolingReleaseRunbookHeading = '## Release Assembly Runbook'
const toolingContractVerifierHeading = '## Assembled contract verifier'
const toolingExampleVerifierHeading = '## Assembled scaffold/example verifier'
const toolingM050VerifierCommand = 'bash scripts/verify-m050-s02.sh'
const toolingM048VerifierCommand = 'bash scripts/verify-m048-s05.sh'
const toolingM049VerifierCommand = 'bash scripts/verify-m049-s05.sh'
const maintainerOnlyMarker = 'mesher/README.md'
const directProofRailMarkers = [
  'scripts/verify-m047-s04.sh',
  'scripts/verify-m047-s05.sh',
  'scripts/verify-m047-s06.sh',
  'e2e_m047_s07',
]
const directRepoRootBackendMarkers = [
  'reference-backend/README.md',
  'meshc test reference-backend',
  'meshc test reference-backend/tests',
  'meshc test reference-backend/tests/config.test.mpl',
  'meshc fmt --check reference-backend',
]
const readmePublicStaleMarkers = [
  'https://meshlang.dev/docs/distributed-proof/',
  'bash scripts/verify-m049-s05.sh',
  'bash scripts/verify-m048-s05.sh',
  'execute_declared_work(...)',
  'Work.execute_declared_work',
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

function validateFirstContactContract(baseRoot) {
  const errors = []
  const readme = readFrom(baseRoot, readmePath)
  const gettingStarted = readFrom(baseRoot, gettingStartedPath)
  const clusteredExample = readFrom(baseRoot, clusteredExamplePath)
  const tooling = readFrom(baseRoot, toolingPath)

  requireIncludes(errors, readmePath, readme, [
    ...starterCommands,
    'honest local starter',
    'shared/deployable',
    readmeLadderIntro,
    readmeClusteredNextStep,
    readmeSqliteNextStep,
    readmePostgresNextStep,
    readmeProofNextStep,
    readmeToolingNextStep,
    readmeMaintainerHeading,
    readmeMaintainerMarker,
    readmeMaintainerVerifierMarker,
  ])

  requireIncludes(errors, gettingStartedPath, gettingStarted, [
    ...starterCommands,
    gettingStartedStarterHeading,
    gettingStartedLadderIntro,
    'honest local starter',
    'shared/deployable',
    currentRepoUrl,
    gettingStartedClusteredNextStep,
    gettingStartedSqliteNextStep,
    gettingStartedPostgresNextStep,
    gettingStartedProofNextStep,
  ])

  requireIncludes(errors, clusteredExamplePath, clusteredExample, [
    'meshc init --clustered hello_cluster',
    'meshc init --template todo-api --db sqlite my_local_todo',
    'meshc init --template todo-api --db postgres my_shared_todo',
    clusteredExampleIntroMarker,
    clusteredExampleStarterHeading,
    clusteredExampleLadderIntro,
    clusteredExampleProofHeading,
    '@cluster pub fn add() -> Int do',
    'Node.start_from_env()',
    'meshc cluster status',
    'meshc cluster continuity',
    'meshc cluster diagnostics',
    sqliteStarterLink,
    postgresStarterLink,
    clusteredExampleProofNextStep,
    clusteredExampleProofPage,
  ])

  requireIncludes(errors, toolingPath, tooling, [
    '## Install the CLI tools',
    '### Update an installed toolchain',
    '## Package Manager',
    toolingWorkflowMarker,
    '### Creating a New Project',
    ...starterCommands,
    'honest local starter',
    'shared/deployable',
    sqliteStarterLink,
    postgresStarterLink,
    'meshc cluster status',
    'meshc cluster continuity',
    'meshc cluster diagnostics',
    toolingStarterLadderIntro,
    toolingProofNextStep,
    'meshc test .',
    'meshc test tests',
    'meshc test tests/example.test.mpl',
    'meshc test --coverage .',
    'meshc fmt --check .',
    '## Editor Support',
    '### Support tiers',
    '### VS Code',
    '### Neovim',
    '### Best-effort editors',
    'bash scripts/verify-m036-s03.sh',
    toolingDocsVerifierHeading,
    toolingM050VerifierCommand,
    toolingReleaseRunbookHeading,
    toolingContractVerifierHeading,
    toolingM048VerifierCommand,
    toolingExampleVerifierHeading,
    toolingM049VerifierCommand,
  ])

  requireExcludes(errors, readmePath, readme, [
    staleRepoUrl,
    maintainerOnlyMarker && false,
  ].filter(Boolean))
  requireExcludes(errors, readmePath, readme, [
    staleRepoUrl,
    ...directRepoRootBackendMarkers,
    ...readmePublicStaleMarkers,
    ...directProofRailMarkers,
  ])
  requireExcludes(errors, gettingStartedPath, gettingStarted, [
    staleRepoUrl,
    maintainerOnlyMarker,
    ...directRepoRootBackendMarkers,
    ...directProofRailMarkers,
  ])
  requireExcludes(errors, clusteredExamplePath, clusteredExample, [
    staleRepoBlobBase,
    maintainerOnlyMarker,
    ...directRepoRootBackendMarkers,
    'execute_declared_work(...)',
    'Work.execute_declared_work',
    ...directProofRailMarkers,
  ])
  requireExcludes(errors, toolingPath, tooling, [
    staleRepoUrl,
    staleRepoBlobBase,
    maintainerOnlyMarker,
    ...directRepoRootBackendMarkers,
    ...directProofRailMarkers,
  ])

  requireOrdered(errors, readmePath, readme, [
    'meshc init hello_mesh',
    ...starterCommands,
    '## Where to go next',
    readmeLadderIntro,
    readmeClusteredNextStep,
    readmeSqliteNextStep,
    readmePostgresNextStep,
    readmeProofNextStep,
    readmeToolingNextStep,
    readmeMaintainerHeading,
    readmeMaintainerMarker,
  ])

  requireOrdered(errors, gettingStartedPath, gettingStarted, [
    '## Hello World',
    gettingStartedStarterHeading,
    ...starterCommands,
    '## What\'s Next?',
    gettingStartedLadderIntro,
    gettingStartedClusteredNextStep,
    gettingStartedSqliteNextStep,
    gettingStartedPostgresNextStep,
    gettingStartedProofNextStep,
  ])

  requireOrdered(errors, clusteredExamplePath, clusteredExample, [
    clusteredExampleIntroMarker,
    'meshc init --clustered hello_cluster',
    clusteredExampleStarterHeading,
    clusteredExampleLadderIntro,
    'meshc init --template todo-api --db sqlite my_local_todo',
    'meshc init --template todo-api --db postgres my_shared_todo',
    clusteredExampleProofNextStep,
    clusteredExampleProofHeading,
    clusteredExampleProofPage,
  ])

  requireOrdered(errors, toolingPath, tooling, [
    '## Install the CLI tools',
    '### Update an installed toolchain',
    '## Package Manager',
    toolingWorkflowMarker,
    '### Creating a New Project',
    ...starterCommands,
    'Inspect a running clustered app with the same operator order used by the scaffold',
    toolingStarterLadderIntro,
    toolingProofNextStep,
    '## Editor Support',
    toolingDocsVerifierHeading,
    toolingM050VerifierCommand,
    toolingReleaseRunbookHeading,
    toolingContractVerifierHeading,
    toolingM048VerifierCommand,
    toolingExampleVerifierHeading,
    toolingM049VerifierCommand,
  ])

  return errors
}

test('current repo publishes the first-contact starter chooser contract', () => {
  const errors = validateFirstContactContract(root)
  assert.deepEqual(errors, [], errors.join('\n'))
})

test('contract fails closed when README loses the explicit examples-first ladder or maintainer-only Mesher handoff', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s02-readme-')
  for (const relativePath of [readmePath, gettingStartedPath, clusteredExamplePath, toolingPath]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  let mutatedReadme = readFrom(tmpRoot, readmePath)
  mutatedReadme = mutatedReadme.replace(readmeLadderIntro, 'Go straight to the backend proof surface after hello-world.')
  mutatedReadme = mutatedReadme.replace(readmeProofNextStep, '- **Production Backend Proof:** https://meshlang.dev/docs/production-backend-proof/ paired with reference-backend/README.md')
  mutatedReadme = mutatedReadme.replace(readmeMaintainerMarker, '`reference-backend/README.md`')
  writeTo(tmpRoot, readmePath, mutatedReadme)

  const errors = validateFirstContactContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes(`README.md missing ${JSON.stringify(readmeLadderIntro)}`) || error.includes(`README.md missing ordered marker ${JSON.stringify(readmeLadderIntro)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`README.md missing ${JSON.stringify(readmeProofNextStep)}`) || error.includes(`README.md missing ordered marker ${JSON.stringify(readmeProofNextStep)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`README.md missing ${JSON.stringify(readmeMaintainerMarker)}`) || error.includes(`README.md missing ordered marker ${JSON.stringify(readmeMaintainerMarker)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`README.md still contains stale text ${JSON.stringify('reference-backend/README.md')}`)), errors.join('\n'))
})

test('contract fails closed when Getting Started loses the chooser heading, repo URL, or examples-first follow-on order', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s02-getting-started-')
  for (const relativePath of [readmePath, gettingStartedPath, clusteredExamplePath, toolingPath]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  let mutatedGettingStarted = readFrom(tmpRoot, gettingStartedPath)
  mutatedGettingStarted = mutatedGettingStarted.replace(
    gettingStartedLadderIntro,
    'Jump from the starter chooser straight into backend proof.',
  )
  mutatedGettingStarted = mutatedGettingStarted.replace(gettingStartedStarterHeading, '## Starter paths')
  mutatedGettingStarted = mutatedGettingStarted.replace(currentRepoUrl, staleRepoUrl)
  mutatedGettingStarted = mutatedGettingStarted.replace(
    gettingStartedProofNextStep,
    '- [Production Backend Proof](/docs/production-backend-proof/) paired with reference-backend/README.md',
  )
  writeTo(tmpRoot, gettingStartedPath, mutatedGettingStarted)

  const errors = validateFirstContactContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/getting-started/index.md missing ${JSON.stringify(gettingStartedStarterHeading)}`) || error.includes(`website/docs/docs/getting-started/index.md missing ordered marker ${JSON.stringify(gettingStartedStarterHeading)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/getting-started/index.md missing ${JSON.stringify(gettingStartedLadderIntro)}`) || error.includes(`website/docs/docs/getting-started/index.md missing ordered marker ${JSON.stringify(gettingStartedLadderIntro)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/getting-started/index.md missing ${JSON.stringify(currentRepoUrl)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/getting-started/index.md still contains stale text ${JSON.stringify(staleRepoUrl)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/getting-started/index.md still contains stale text ${JSON.stringify('reference-backend/README.md')}`)), errors.join('\n'))
})

test('contract fails closed when Clustered Example loses scaffold-first starter truth or routes directly to repo-root backend guidance', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s02-clustered-example-')
  for (const relativePath of [readmePath, gettingStartedPath, clusteredExamplePath, toolingPath]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  let mutatedClusteredExample = readFrom(tmpRoot, clusteredExamplePath)
  mutatedClusteredExample = mutatedClusteredExample.replace(clusteredExampleIntroMarker, 'Use this page as a short bridge to the backend runbook.')
  mutatedClusteredExample = mutatedClusteredExample.replace(clusteredExampleStarterHeading, '## Pick the next cluster step')
  mutatedClusteredExample = mutatedClusteredExample.replace(clusteredExampleLadderIntro, 'Jump to backend proof as soon as the scaffold builds.')
  mutatedClusteredExample = mutatedClusteredExample.replace(
    'meshc init --template todo-api --db sqlite my_local_todo',
    'meshc init --template todo-api my_local_todo',
  )
  mutatedClusteredExample = mutatedClusteredExample.replace(clusteredExampleProofNextStep, '- [reference-backend/README.md](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md)')
  mutatedClusteredExample = mutatedClusteredExample.replace(clusteredExampleProofHeading, '## Need direct proof rails right now?')
  mutatedClusteredExample = mutatedClusteredExample.replaceAll(clusteredExampleProofPage, 'bash scripts/verify-m047-s04.sh')
  writeTo(tmpRoot, clusteredExamplePath, mutatedClusteredExample)

  const errors = validateFirstContactContract(tmpRoot)
  const joinedErrors = errors.join('\n')
  assert.match(joinedErrors, /website\/docs\/docs\/getting-started\/clustered-example\/index\.md (missing|missing ordered marker) "This page stays on that scaffold first\./)
  assert.match(joinedErrors, /website\/docs\/docs\/getting-started\/clustered-example\/index\.md (missing|missing ordered marker) "## After the scaffold, pick the follow-on starter"/)
  assert.match(joinedErrors, /website\/docs\/docs\/getting-started\/clustered-example\/index\.md (missing|missing ordered marker) "Take the public follow-on ladder in order: honest local SQLite starter, serious shared\/deployable PostgreSQL starter, then Production Backend Proof only when you need the maintainer-facing deeper backend proof\."/)
  assert.match(joinedErrors, /website\/docs\/docs\/getting-started\/clustered-example\/index\.md (missing|missing ordered marker) "meshc init --template todo-api --db sqlite my_local_todo"/)
  assert.match(joinedErrors, /website\/docs\/docs\/getting-started\/clustered-example\/index\.md still contains stale text "reference-backend\/README\.md"/)
  assert.match(joinedErrors, /website\/docs\/docs\/getting-started\/clustered-example\/index\.md still contains stale text "scripts\/verify-m047-s04\.sh"/)
})

test('contract fails closed when Tooling reintroduces repo-root backend day-one commands or loses the examples-first ladder wording', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s02-tooling-')
  for (const relativePath of [readmePath, gettingStartedPath, clusteredExamplePath, toolingPath]) {
    copyRepoFile(tmpRoot, relativePath)
  }

  let mutatedTooling = readFrom(tmpRoot, toolingPath)
  mutatedTooling = mutatedTooling.replace(toolingWorkflowMarker, 'Tooling jumps straight from install into backend proof.')
  mutatedTooling = mutatedTooling.replace(toolingStarterLadderIntro, 'After that CLI order, branch intentionally:')
  mutatedTooling = mutatedTooling.replace('### Support tiers', '### Editor tiers')
  mutatedTooling = mutatedTooling.replace(toolingDocsVerifierHeading, '## Docs verifier')
  mutatedTooling = mutatedTooling.replace(toolingProofNextStep, '- [Production Backend Proof](/docs/production-backend-proof/) paired with reference-backend/README.md')
  mutatedTooling = mutatedTooling.replace('meshc test .', 'meshc test reference-backend')
  mutatedTooling = mutatedTooling.replace('meshc test tests', 'meshc test reference-backend/tests')
  mutatedTooling = mutatedTooling.replace('meshc test tests/example.test.mpl', 'meshc test reference-backend/tests/config.test.mpl')
  mutatedTooling = mutatedTooling.replace('meshc test --coverage .', 'meshc test --coverage reference-backend')
  mutatedTooling = mutatedTooling.replace('meshc fmt --check .', 'meshc fmt --check reference-backend')
  mutatedTooling = mutatedTooling.replace(toolingM050VerifierCommand, toolingM048VerifierCommand)
  mutatedTooling = mutatedTooling.replace(postgresStarterLink, `${staleRepoBlobBase}examples/todo-postgres/README.md`)
  writeTo(tmpRoot, toolingPath, mutatedTooling)

  const errors = validateFirstContactContract(tmpRoot)
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md missing ${JSON.stringify(toolingWorkflowMarker)}`) || error.includes(`website/docs/docs/tooling/index.md missing ordered marker ${JSON.stringify(toolingWorkflowMarker)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md missing ${JSON.stringify(toolingStarterLadderIntro)}`) || error.includes(`website/docs/docs/tooling/index.md missing ordered marker ${JSON.stringify(toolingStarterLadderIntro)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md missing ${JSON.stringify(toolingDocsVerifierHeading)}`) || error.includes(`website/docs/docs/tooling/index.md missing ordered marker ${JSON.stringify(toolingDocsVerifierHeading)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md missing ${JSON.stringify(toolingM050VerifierCommand)}`) || error.includes(`website/docs/docs/tooling/index.md missing ordered marker ${JSON.stringify(toolingM050VerifierCommand)}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md still contains stale text ${JSON.stringify('reference-backend/README.md')}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md still contains stale text ${JSON.stringify('meshc test reference-backend')}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md still contains stale text ${JSON.stringify('meshc fmt --check reference-backend')}`)), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes(`website/docs/docs/tooling/index.md still contains stale text ${JSON.stringify(staleRepoBlobBase)}`)), errors.join('\n'))
})
