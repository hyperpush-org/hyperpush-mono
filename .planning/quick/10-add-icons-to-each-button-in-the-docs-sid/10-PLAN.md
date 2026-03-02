---
phase: quick-10
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - website/docs/.vitepress/theme/composables/useSidebar.ts
  - website/docs/.vitepress/config.mts
  - website/docs/.vitepress/theme/components/docs/DocsSidebarItem.vue
autonomous: true
requirements: []
must_haves:
  truths:
    - "Each sidebar link has a small icon to its left"
    - "Active sidebar item icon is visible and styled consistently with text"
    - "Icons come from the already-installed lucide-vue-next package"
  artifacts:
    - path: "website/docs/.vitepress/theme/composables/useSidebar.ts"
      provides: "SidebarItem interface with optional icon field"
      contains: "icon?: string"
    - path: "website/docs/.vitepress/theme/components/docs/DocsSidebarItem.vue"
      provides: "Renders icon component inline before item text"
    - path: "website/docs/.vitepress/config.mts"
      provides: "Every leaf sidebar item has an icon name assigned"
  key_links:
    - from: "website/docs/.vitepress/config.mts"
      to: "website/docs/.vitepress/theme/components/docs/DocsSidebarItem.vue"
      via: "icon property on SidebarItem passed through VitePress theme config"
---

<objective>
Add a Lucide icon to every sidebar navigation link in the docs site.

Purpose: Improve visual hierarchy and scannability of the docs sidebar — each item should have a recognizable icon that matches its content topic.
Output: Updated SidebarItem type, icon-aware DocsSidebarItem component, icon assignments on every config entry.
</objective>

<execution_context>
@/Users/sn0w/.claude/get-shit-done/workflows/execute-plan.md
@/Users/sn0w/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

Relevant files:
- website/docs/.vitepress/config.mts — sidebar config with all items
- website/docs/.vitepress/theme/composables/useSidebar.ts — SidebarItem interface
- website/docs/.vitepress/theme/components/docs/DocsSidebarItem.vue — renders each link

Tech: lucide-vue-next ^0.564.0 is already installed (see website/package.json).

<interfaces>
<!-- Current SidebarItem (useSidebar.ts) -->
export interface SidebarItem {
  text?: string
  link?: string
  items?: SidebarItem[]
  collapsed?: boolean
  base?: string
  docFooterText?: string
}

<!-- Current DocsSidebarItem.vue template — renders a plain anchor tag, no icon -->
<a :href="item.link" class="block rounded-md px-2 py-1.5 text-[13px] ...">
  {{ item.text }}
</a>

<!-- Current sidebar leaf items from config.mts -->
{ text: 'Introduction',         link: '/docs/getting-started/' }
{ text: 'Language Basics',      link: '/docs/language-basics/' }
{ text: 'Type System',          link: '/docs/type-system/' }
{ text: 'Iterators',            link: '/docs/iterators/' }
{ text: 'Concurrency',          link: '/docs/concurrency/' }
{ text: 'Web',                  link: '/docs/web/' }
{ text: 'Databases',            link: '/docs/databases/' }
{ text: 'Distributed Actors',   link: '/docs/distributed/' }
{ text: 'Developer Tools',      link: '/docs/tooling/' }
{ text: 'Standard Library',     link: '/docs/stdlib/' }
{ text: 'Testing',              link: '/docs/testing/' }
{ text: 'Syntax Cheatsheet',    link: '/docs/cheatsheet/' }
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Extend SidebarItem type and wire icon rendering in DocsSidebarItem</name>
  <files>
    website/docs/.vitepress/theme/composables/useSidebar.ts
    website/docs/.vitepress/theme/components/docs/DocsSidebarItem.vue
  </files>
  <action>
    **useSidebar.ts** — Add `icon?: string` field to the SidebarItem interface. No other changes needed.

    **DocsSidebarItem.vue** — Use lucide-vue-next's dynamic component pattern to render the icon. Import `defineAsyncComponent` is not needed — lucide-vue-next exports every icon as a named component. Use the `<component :is="...">` approach with a computed lookup.

    Concrete implementation:

    ```vue
    <script setup lang="ts">
    import { computed } from 'vue'
    import { useData } from 'vitepress'
    import { isActive, type SidebarItem } from '@/composables/useSidebar'
    import * as LucideIcons from 'lucide-vue-next'

    const props = defineProps<{
      item: SidebarItem
    }>()

    const { page } = useData()
    const active = computed(() => isActive(page.value.relativePath, props.item.link))

    const iconComponent = computed(() => {
      if (!props.item.icon) return null
      return (LucideIcons as Record<string, unknown>)[props.item.icon] ?? null
    })
    </script>

    <template>
      <div>
        <a
          :href="item.link"
          class="flex items-center gap-2 rounded-md px-2 py-1.5 text-[13px] transition-colors"
          :class="[
            active
              ? 'bg-accent text-foreground font-semibold'
              : 'text-muted-foreground hover:text-foreground hover:bg-accent',
          ]"
        >
          <component
            v-if="iconComponent"
            :is="iconComponent"
            class="size-3.5 shrink-0"
          />
          {{ item.text }}
        </a>
        <!-- Recursive children with left padding -->
        <ul v-if="item.items?.length" class="flex flex-col gap-0.5 pl-3 mt-0.5">
          <li v-for="child in item.items" :key="child.text">
            <DocsSidebarItem :item="child" />
          </li>
        </ul>
      </div>
    </template>
    ```

    Key decisions:
    - `import * as LucideIcons` gives access to every icon by PascalCase name; no bundler issue since tree-shaking still works per export
    - Icon size `size-3.5` (14px) keeps it compact alongside 13px text
    - `shrink-0` prevents icon squishing on narrow sidebars
    - `flex items-center gap-2` replaces `block` on the anchor so icon and text align
    - Items without `icon` set render unchanged (icon slot stays empty)
  </action>
  <verify>npx tsc --noEmit -p /Users/sn0w/Documents/dev/mesh/website/docs/.vitepress/tsconfig.json 2>/dev/null || echo "no tsconfig — skip type check"</verify>
  <done>DocsSidebarItem renders an icon when item.icon is set; items without icon still render correctly; no TypeScript errors</done>
</task>

<task type="auto">
  <name>Task 2: Assign icons to every sidebar item in config.mts</name>
  <files>
    website/docs/.vitepress/config.mts
  </files>
  <action>
    Update every leaf item in the `/docs/` sidebar to include an `icon` property. Use lucide-vue-next PascalCase names.

    Mapping (chosen for semantic fit):
    - Introduction         → `BookOpen`
    - Language Basics      → `Code2`
    - Type System          → `Shapes`
    - Iterators            → `Repeat`
    - Concurrency          → `Workflow`
    - Web                  → `Globe`
    - Databases            → `Database`
    - Distributed Actors   → `Network`
    - Developer Tools      → `Wrench`
    - Standard Library     → `Library`
    - Testing              → `FlaskConical`
    - Syntax Cheatsheet    → `ClipboardList`

    Example diff for the Getting Started section:
    ```ts
    {
      text: 'Getting Started',
      items: [
        { text: 'Introduction', link: '/docs/getting-started/', icon: 'BookOpen' },
      ],
    },
    ```

    Apply the same pattern to all 12 leaf items. Do not add icons to group headers (the `text`-only objects with `items` arrays) — those render via DocsSidebarGroup.vue which uses plain text.

    Also extend the VitePress `themeConfig` type declaration if TypeScript complains — the `SidebarItem` from VitePress's own types won't have `icon`. Cast each item object with `as any` inline, or add a short module augmentation at the top of config.mts:

    ```ts
    declare module 'vitepress' {
      interface UserConfig {
        // No-op — using custom SidebarItem type from composable
      }
    }
    ```

    Actually the simpler fix: the sidebar array is typed via VitePress's own `DefaultTheme.SidebarItem`. Since the custom `DocsSidebarItem.vue` reads from `item.icon` directly (and our `useSidebar.ts` SidebarItem already has the field), VitePress just passes the raw config object through. TypeScript may warn about the extra property — suppress with `// @ts-expect-error` on each icon line, or cast the entire sidebar array `as any`. Use `as any` on the full sidebar value (cleanest).
  </action>
  <verify>cd /Users/sn0w/Documents/dev/mesh/website && npm run build 2>&1 | tail -20</verify>
  <done>Build succeeds with no errors; all 12 sidebar items have icon names assigned in config.mts</done>
</task>

</tasks>

<verification>
After both tasks:
1. Run `cd /Users/sn0w/Documents/dev/mesh/website && npm run build` — build must complete without errors
2. Run `npm run preview` and open http://localhost:4173/docs/ to confirm icons appear in sidebar
3. Verify icons render on both active and inactive items
4. Verify items without explicit icon set (group headers) are unaffected
</verification>

<success_criteria>
- Every leaf sidebar link in the docs has a small Lucide icon to its left
- Build passes without errors or TypeScript failures
- Icons are visually aligned with text (flex row, 14px, muted color matching text)
- No visual regression on mobile sidebar (MobileSidebar.vue also uses DocsSidebarItem)
</success_criteria>

<output>
After completion, create `.planning/quick/10-add-icons-to-each-button-in-the-docs-sid/10-SUMMARY.md`
</output>
