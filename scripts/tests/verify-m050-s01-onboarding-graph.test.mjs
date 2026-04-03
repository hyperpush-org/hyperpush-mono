import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(scriptDir, '..', '..')

const filePaths = {
  config: 'website/docs/.vitepress/config.mts',
  prevNext: 'website/docs/.vitepress/theme/composables/usePrevNext.ts',
  productionProof: 'website/docs/docs/production-backend-proof/index.md',
  distributedProof: 'website/docs/docs/distributed-proof/index.md',
}

const proofLinks = [
  '/docs/distributed-proof/',
  '/docs/production-backend-proof/',
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

function replaceOrThrow(text, searchValue, replaceValue) {
  const replaced = text.replace(searchValue, replaceValue)
  assert.notEqual(replaced, text, `expected to replace ${String(searchValue)}`)
  return replaced
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
      errors.push(`${relativePath} still contains ${JSON.stringify(needle)}`)
    }
  }
}

function extractFrontmatter(relativePath, text) {
  const match = text.match(/^---\n([\s\S]*?)\n---/)
  assert.ok(match, `unable to locate frontmatter in ${relativePath}`)
  return match[1]
}

function extractDocsSidebarBlock(configSource) {
  const match = configSource.match(/sidebar:\s*{\s*'\/docs\/': \[(?<block>[\s\S]*?)\n\s*\],\s*\n\s*},\s*\n\s*outline:/)
  assert.ok(match?.groups?.block, 'unable to locate /docs/ sidebar in website/docs/.vitepress/config.mts')
  return match.groups.block
}

function parseSidebarGroups(configSource) {
  const sidebarBlock = extractDocsSidebarBlock(configSource)
  const groups = []
  const groupPattern = /{\s*text:\s*'(?<name>[^']+)'[\s\S]*?items:\s*\[(?<items>[\s\S]*?)\]\s*,\s*}/g

  for (const match of sidebarBlock.matchAll(groupPattern)) {
    const name = match.groups?.name
    const itemsBlock = match.groups?.items
    if (!name || itemsBlock == null) continue
    groups.push({
      name,
      items: parseSidebarItems(itemsBlock),
    })
  }

  assert.ok(groups.length > 0, 'unable to parse any /docs/ sidebar groups from website/docs/.vitepress/config.mts')
  return groups
}

function parseSidebarItems(itemsBlock) {
  const items = []
  const itemPattern = /{\s*text:\s*'(?<text>[^']+)'\s*,\s*link:\s*'(?<link>[^']+)'(?<tail>[\s\S]*?)}\s*as any/g

  for (const match of itemsBlock.matchAll(itemPattern)) {
    const text = match.groups?.text
    const link = match.groups?.link
    const tail = match.groups?.tail ?? ''
    if (!text || !link) continue
    items.push({
      text,
      link,
      includeInFooter: /includeInFooter:\s*false/.test(tail)
        ? false
        : undefined,
    })
  }

  return items
}

function getGroup(groups, name, errors) {
  const group = groups.find((candidate) => candidate.name === name)
  if (!group) {
    errors.push(`${filePaths.config} missing sidebar group ${JSON.stringify(name)}`)
    return null
  }
  return group
}

function requireLinksEqual(errors, label, items, expectedLinks) {
  const actualLinks = items.map((item) => item.link)
  if (JSON.stringify(actualLinks) !== JSON.stringify(expectedLinks)) {
    errors.push(`${label} links drifted: expected ${JSON.stringify(expectedLinks)} but found ${JSON.stringify(actualLinks)}`)
  }
}

function flattenFooterCandidates(groups) {
  const allItems = groups.flatMap((group) => group.items)
  const seen = new Set()
  return allItems.filter((item) => {
    if (item.includeInFooter === false) return false
    const normalized = item.link.replace(/[?#].*$/, '')
    if (seen.has(normalized)) return false
    seen.add(normalized)
    return true
  })
}

function normalizeDocPath(targetPath) {
  const normalized = targetPath.replace(/(index)?\.(md|html)$/, '').replace(/\/$/, '')
  return normalized.startsWith('/') ? normalized : `/${normalized}`
}

function resolveFooterLinks(candidates, currentPath) {
  const normalizedCurrent = normalizeDocPath(currentPath)
  const index = candidates.findIndex((item) => normalizeDocPath(item.link) === normalizedCurrent)
  if (index === -1) {
    return { prev: undefined, next: undefined }
  }
  return {
    prev: candidates[index - 1],
    next: candidates[index + 1],
  }
}

function validateOnboardingGraph(baseRoot) {
  const errors = []
  const configSource = readFrom(baseRoot, filePaths.config)
  const prevNextSource = readFrom(baseRoot, filePaths.prevNext)
  const productionProof = readFrom(baseRoot, filePaths.productionProof)
  const distributedProof = readFrom(baseRoot, filePaths.distributedProof)

  const groups = parseSidebarGroups(configSource)
  const groupNames = groups.map((group) => group.name)
  const gettingStarted = getGroup(groups, 'Getting Started', errors)
  const distribution = getGroup(groups, 'Distribution', errors)
  const reference = getGroup(groups, 'Reference', errors)
  const proofSurfaces = getGroup(groups, 'Proof Surfaces', errors)

  if (gettingStarted) {
    requireLinksEqual(errors, 'Getting Started', gettingStarted.items, [
      '/docs/getting-started/',
      '/docs/getting-started/clustered-example/',
    ])
  }

  if (distribution) {
    requireLinksEqual(errors, 'Distribution', distribution.items, [
      '/docs/distributed/',
    ])
  }

  if (proofSurfaces) {
    requireLinksEqual(errors, 'Proof Surfaces', proofSurfaces.items, proofLinks)
    for (const item of proofSurfaces.items) {
      if (item.includeInFooter !== false) {
        errors.push(`${filePaths.config} proof sidebar item ${JSON.stringify(item.link)} must set includeInFooter: false`)
      }
    }
  }

  const proofSurfacesIndex = groupNames.indexOf('Proof Surfaces')
  const referenceIndex = groupNames.indexOf('Reference')
  if (proofSurfacesIndex !== -1 && referenceIndex !== -1 && proofSurfacesIndex <= referenceIndex) {
    errors.push(`${filePaths.config} places "Proof Surfaces" before "Reference" instead of keeping proof links secondary`)
  }
  if (proofSurfacesIndex !== -1 && proofSurfacesIndex !== groupNames.length - 1) {
    errors.push(`${filePaths.config} must keep "Proof Surfaces" as the final /docs/ sidebar group`)
  }

  const allItems = groups.flatMap((group) => group.items.map((item) => ({
    group: group.name,
    ...item,
  })))

  for (const proofLink of proofLinks) {
    const matches = allItems.filter((item) => item.link === proofLink)
    if (matches.length !== 1) {
      errors.push(`${filePaths.config} ${proofLink} appears ${matches.length} times in the /docs/ sidebar`)
      continue
    }
    if (matches[0].group !== 'Proof Surfaces') {
      errors.push(`${filePaths.config} ${proofLink} is in ${JSON.stringify(matches[0].group)} instead of "Proof Surfaces"`)
    }
  }

  requireIncludes(errors, filePaths.prevNext, prevNextSource, [
    'link.includeInFooter !== false',
    'isSamePage(page.value.relativePath, link.link)',
    'if (index === -1) return { prev: undefined, next: undefined }',
    'return normalizedCurrent === normalizedCandidate',
  ])
  requireExcludes(errors, filePaths.prevNext, prevNextSource, [
    "import { isActive } from './useSidebar'",
    'isActive(page.value.relativePath, link.link)',
    'normalizedCurrent.startsWith(',
  ])

  const footerCandidates = flattenFooterCandidates(groups)
  const gettingStartedFooter = resolveFooterLinks(footerCandidates, '/docs/getting-started/')
  if (gettingStartedFooter.next?.link !== '/docs/getting-started/clustered-example/') {
    errors.push(`footer graph drifted: Getting Started next link should be "/docs/getting-started/clustered-example/" but found ${JSON.stringify(gettingStartedFooter.next?.link)}`)
  }

  const clusteredExampleFooter = resolveFooterLinks(
    footerCandidates,
    '/docs/getting-started/clustered-example/',
  )
  if (clusteredExampleFooter.prev?.link !== '/docs/getting-started/') {
    errors.push(`footer graph drifted: Clustered Example prev link should be "/docs/getting-started/" but found ${JSON.stringify(clusteredExampleFooter.prev?.link)}`)
  }
  if (clusteredExampleFooter.next?.link !== '/docs/language-basics/') {
    errors.push(`footer graph drifted: Clustered Example next link should be "/docs/language-basics/" but found ${JSON.stringify(clusteredExampleFooter.next?.link)}`)
  }
  if (clusteredExampleFooter.next?.link === '/docs/getting-started/clustered-example/') {
    errors.push('footer graph drifted: Clustered Example still resolves to itself')
  }
  if (proofLinks.includes(clusteredExampleFooter.next?.link)) {
    errors.push(`footer graph drifted: Clustered Example next link must not jump directly to a proof page (${clusteredExampleFooter.next.link})`)
  }

  for (const proofLink of proofLinks) {
    const proofFooter = resolveFooterLinks(footerCandidates, proofLink)
    if (proofFooter.prev || proofFooter.next) {
      errors.push(`footer graph drifted: ${proofLink} should be excluded from footer candidates but resolved ${JSON.stringify(proofFooter)}`)
    }
  }

  const productionFrontmatter = extractFrontmatter(filePaths.productionProof, productionProof)
  requireIncludes(errors, `${filePaths.productionProof} frontmatter`, productionFrontmatter, [
    'prev: false',
    'next: false',
  ])

  const distributedFrontmatter = extractFrontmatter(filePaths.distributedProof, distributedProof)
  requireIncludes(errors, `${filePaths.distributedProof} frontmatter`, distributedFrontmatter, [
    'prev: false',
    'next: false',
  ])

  return errors
}

test('current repo publishes the onboarding graph with proof pages kept secondary and out of the footer chain', () => {
  const errors = validateOnboardingGraph(root)
  assert.deepEqual(errors, [], errors.join('\n'))
})

test('contract fails closed when a proof page drifts back into a primary sidebar group', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s01-primary-drift-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const relativePath = filePaths.config
  let mutated = readFrom(tmpRoot, relativePath)
  mutated = replaceOrThrow(
    mutated,
    "{ text: 'Distributed Actors', link: '/docs/distributed/', icon: 'Network' } as any,",
    "{ text: 'Distributed Proof', link: '/docs/distributed-proof/', icon: 'ShieldCheck' } as any,",
  )
  writeTo(tmpRoot, relativePath, mutated)

  const errors = validateOnboardingGraph(tmpRoot)
  assert.ok(errors.some((error) => error.includes('Distribution links drifted')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('/docs/distributed-proof/ appears 2 times')), errors.join('\n'))
})

test('contract fails closed when the footer matcher regresses to prefix matching', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s01-footer-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const relativePath = filePaths.prevNext
  let mutated = readFrom(tmpRoot, relativePath)
  mutated = replaceOrThrow(
    mutated,
    'return normalizedCurrent === normalizedCandidate',
    "return normalizedCurrent === normalizedCandidate || normalizedCurrent.startsWith(normalizedCandidate + '/')",
  )
  writeTo(tmpRoot, relativePath, mutated)

  const errors = validateOnboardingGraph(tmpRoot)
  assert.ok(errors.some((error) => error.includes('normalizedCurrent.startsWith(')), errors.join('\n'))
})

test('contract fails closed when proof pages lose their footer opt-out markers', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s01-proof-opt-out-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  let configSource = readFrom(tmpRoot, filePaths.config)
  configSource = replaceOrThrow(configSource, 'includeInFooter: false,', '')
  writeTo(tmpRoot, filePaths.config, configSource)

  let proofPage = readFrom(tmpRoot, filePaths.productionProof)
  proofPage = replaceOrThrow(proofPage, 'next: false\n', '')
  writeTo(tmpRoot, filePaths.productionProof, proofPage)

  const errors = validateOnboardingGraph(tmpRoot)
  assert.ok(errors.some((error) => error.includes('must set includeInFooter: false')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('production-backend-proof/index.md frontmatter missing "next: false"')), errors.join('\n'))
})

test('contract fails closed when a proof-surface route is typoed', (t) => {
  const tmpRoot = mkTmpDir(t, 'verify-m050-s01-typo-')
  for (const relativePath of Object.values(filePaths)) {
    copyRepoFile(tmpRoot, relativePath)
  }

  const relativePath = filePaths.config
  let mutated = readFrom(tmpRoot, relativePath)
  mutated = replaceOrThrow(
    mutated,
    "/docs/production-backend-proof/",
    '/docs/production-backend-proofs/',
  )
  writeTo(tmpRoot, relativePath, mutated)

  const errors = validateOnboardingGraph(tmpRoot)
  assert.ok(errors.some((error) => error.includes('Proof Surfaces links drifted')), errors.join('\n'))
  assert.ok(errors.some((error) => error.includes('/docs/production-backend-proof/ appears 0 times')), errors.join('\n'))
})
