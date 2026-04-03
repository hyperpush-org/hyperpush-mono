import { computed } from 'vue'
import { useData } from 'vitepress'

interface FlatLink {
  text: string
  link: string
  docFooterText?: string
  includeInFooter?: boolean
}

export function usePrevNext() {
  const { page, theme, frontmatter } = useData()

  return computed(() => {
    const sidebarConfig = theme.value.sidebar
    if (!sidebarConfig) return { prev: undefined, next: undefined }

    // Resolve current sidebar
    const relativePath = page.value.relativePath
    const sidebar = resolveSidebar(sidebarConfig, relativePath)

    // Flatten all footer-eligible links from sidebar
    const links = flattenSidebarLinks(sidebar).filter(
      (link) => link.includeInFooter !== false,
    )
    const candidates = uniqBy(links, (l) => l.link.replace(/[?#].*$/, ''))

    // Find current page index using exact page matching
    const index = candidates.findIndex((link) =>
      isSamePage(page.value.relativePath, link.link),
    )

    if (index === -1) return { prev: undefined, next: undefined }

    return {
      prev:
        frontmatter.value.prev === false
          ? undefined
          : {
              text:
                candidates[index - 1]?.docFooterText ??
                candidates[index - 1]?.text,
              link: candidates[index - 1]?.link,
            },
      next:
        frontmatter.value.next === false
          ? undefined
          : {
              text:
                candidates[index + 1]?.docFooterText ??
                candidates[index + 1]?.text,
              link: candidates[index + 1]?.link,
            },
    }
  })
}

function flattenSidebarLinks(items: any[]): FlatLink[] {
  const links: FlatLink[] = []
  function recurse(items: any[]) {
    for (const item of items) {
      if (item.text && item.link) {
        links.push({
          text: item.text,
          link: item.link,
          docFooterText: item.docFooterText,
          includeInFooter: item.includeInFooter,
        })
      }
      if (item.items) recurse(item.items)
    }
  }
  recurse(items)
  return links
}

function resolveSidebar(sidebar: any, relativePath: string): any[] {
  if (Array.isArray(sidebar)) return sidebar
  const path = relativePath.startsWith('/')
    ? relativePath
    : `/${relativePath}`
  const dir = Object.keys(sidebar)
    .sort((a, b) => b.split('/').length - a.split('/').length)
    .find((d) => path.startsWith(d.startsWith('/') ? d : `/${d}`))
  return dir ? sidebar[dir] : []
}

function normalizeDocPath(path: string): string {
  return ensureStartingSlash(
    path.replace(/(index)?\.(md|html)$/, '').replace(/\/$/, ''),
  )
}

function isSamePage(currentPath: string, candidatePath?: string): boolean {
  if (!candidatePath) return false
  const normalizedCurrent = normalizeDocPath(currentPath)
  const normalizedCandidate = normalizeDocPath(candidatePath)
  return normalizedCurrent === normalizedCandidate
}

function ensureStartingSlash(path: string): string {
  return path.startsWith('/') ? path : `/${path}`
}

function uniqBy<T>(arr: T[], fn: (item: T) => string): T[] {
  const seen = new Set<string>()
  return arr.filter((item) => {
    const k = fn(item)
    return seen.has(k) ? false : (seen.add(k), true)
  })
}
