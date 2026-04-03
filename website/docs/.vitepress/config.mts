import { defineConfig } from 'vitepress'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'
import meshGrammar from '../../../tools/editors/vscode-mesh/syntaxes/mesh.tmLanguage.json'
import meshLight from './theme/shiki/mesh-light.json'
import meshDark from './theme/shiki/mesh-dark.json'

export default defineConfig({
  title: 'Mesh',
  description: 'The Mesh Programming Language',

  // Built-in dark mode with FOUC prevention
  appearance: true,

  // Enable clean URLs
  cleanUrls: true,

  // Enable git-based last-updated timestamps
  lastUpdated: true,

  // Generate sitemap
  sitemap: {
    hostname: 'https://meshlang.dev',
  },

  // Site-wide SEO defaults
  head: [
    ['script', {}, `;(() => {
      const s = localStorage.getItem('vitepress-theme-appearance')
      if (!s || s === 'auto') {
        document.documentElement.classList.remove('dark')
        localStorage.setItem('vitepress-theme-appearance', 'light')
      }
    })()`],
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo-icon-black.svg', media: '(prefers-color-scheme: light)' }],
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo-icon-white.svg', media: '(prefers-color-scheme: dark)' }],
    ['meta', { name: 'theme-color', content: '#ffffff' }],
    ['meta', { property: 'og:site_name', content: 'Mesh Programming Language' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
  ],

  // Per-page dynamic SEO meta tags
  transformPageData(pageData) {
    const canonicalUrl = `https://meshlang.dev/${pageData.relativePath}`
      .replace(/index\.md$/, '')
      .replace(/\.md$/, '.html')
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push(
      ['link', { rel: 'canonical', href: canonicalUrl }],
      ['meta', { property: 'og:title', content: pageData.title ? `${pageData.title} | Mesh` : 'Mesh Programming Language' }],
      ['meta', { property: 'og:description', content: pageData.description || 'Expressive, concurrent, type-safe programming language.' }],
      ['meta', { property: 'og:url', content: canonicalUrl }],
      ['meta', { property: 'og:type', content: 'article' }],
    )
  },

  markdown: {
    languages: [
      {
        ...(meshGrammar as any),
        name: 'mesh',
      },
    ],
    theme: {
      light: meshLight as any,
      dark: meshDark as any,
    },
  },

  themeConfig: {
    nav: [
      { text: 'Docs', link: '/docs/' },
      { text: 'Packages', link: '/packages/' },
    ],
    search: { provider: 'local' },
    editLink: {
      pattern: 'https://github.com/snowdamiz/mesh-lang/edit/main/website/docs/:path',
      text: 'Edit this page on GitHub',
    },
    meshVersion: '14.0',
    sidebar: {
      '/docs/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Introduction', link: '/docs/getting-started/', icon: 'BookOpen' } as any,
            { text: 'Clustered Example', link: '/docs/getting-started/clustered-example/', icon: 'Network' } as any,
          ],
        },
        {
          text: 'Language Guide',
          collapsed: false,
          items: [
            { text: 'Language Basics', link: '/docs/language-basics/', icon: 'Code2' } as any,
            { text: 'Type System', link: '/docs/type-system/', icon: 'Shapes' } as any,
            { text: 'Iterators', link: '/docs/iterators/', icon: 'Repeat' } as any,
            { text: 'Concurrency', link: '/docs/concurrency/', icon: 'Workflow' } as any,
          ],
        },
        {
          text: 'Web & Networking',
          collapsed: false,
          items: [
            { text: 'Web', link: '/docs/web/', icon: 'Globe' } as any,
          ],
        },
        {
          text: 'Data',
          collapsed: false,
          items: [
            { text: 'Databases', link: '/docs/databases/', icon: 'Database' } as any,
          ],
        },
        {
          text: 'Distribution',
          collapsed: false,
          items: [
            { text: 'Distributed Actors', link: '/docs/distributed/', icon: 'Network' } as any,
          ],
        },
        {
          text: 'Tooling',
          collapsed: false,
          items: [
            { text: 'Developer Tools', link: '/docs/tooling/', icon: 'Wrench' } as any,
          ],
        },
        {
          text: 'Standard Library',
          collapsed: false,
          items: [
            { text: 'Standard Library', link: '/docs/stdlib/', icon: 'Library' } as any,
            { text: 'Testing', link: '/docs/testing/', icon: 'FlaskConical' } as any,
          ],
        },
        {
          text: 'Reference',
          collapsed: false,
          items: [
            { text: 'Syntax Cheatsheet', link: '/docs/cheatsheet/', icon: 'ClipboardList' } as any,
          ],
        },
        {
          text: 'Proof Surfaces',
          collapsed: false,
          items: [
            {
              text: 'Distributed Proof',
              link: '/docs/distributed-proof/',
              icon: 'ShieldCheck',
              includeInFooter: false,
            } as any,
            {
              text: 'Production Backend Proof',
              link: '/docs/production-backend-proof/',
              icon: 'ShieldCheck',
              includeInFooter: false,
            } as any,
          ],
        },
      ],
    },
    outline: { level: [2, 3], label: 'On this page' },
  },

  vite: {
    plugins: [
      tailwindcss(),
    ],
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './theme'),
      },
    },
  },
})
