import { getDb } from './db'
import type { BlogPost, CreatePostInput, UpdatePostInput } from './types'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function slugify(str: string): string {
  return str
    .toLowerCase()
    .replace(/[^\w\s-]/g, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .trim()
    .slice(0, 80)
}

export function estimateReadTime(html: string): number {
  const text = html.replace(/<[^>]*>/g, ' ').replace(/\s+/g, ' ').trim()
  const words = text.split(' ').filter(Boolean).length
  return Math.max(1, Math.ceil(words / 200))
}

function ensureUniqueSlug(base: string, excludeId?: number): string {
  const db = getDb()
  let slug = base
  let i = 0
  while (true) {
    const q = excludeId
      ? db.prepare('SELECT id FROM posts WHERE slug = ? AND id != ?').get(slug, excludeId)
      : db.prepare('SELECT id FROM posts WHERE slug = ?').get(slug)
    if (!q) return slug
    i++
    slug = `${base}-${i}`
  }
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------
export function listPosts(opts?: {
  status?: 'draft' | 'published' | 'all'
  limit?: number
  offset?: number
}): BlogPost[] {
  const db = getDb()
  const { status = 'published', limit = 50, offset = 0 } = opts ?? {}

  if (status === 'all') {
    return db
      .prepare(
        'SELECT * FROM posts ORDER BY COALESCE(published_at, created_at) DESC LIMIT ? OFFSET ?',
      )
      .all(limit, offset) as BlogPost[]
  }

  return db
    .prepare(
      'SELECT * FROM posts WHERE status = ? ORDER BY published_at DESC LIMIT ? OFFSET ?',
    )
    .all(status, limit, offset) as BlogPost[]
}

export function getPostBySlug(slug: string): BlogPost | null {
  const db = getDb()
  return (db.prepare('SELECT * FROM posts WHERE slug = ?').get(slug) as BlogPost) ?? null
}

export function getPostById(id: number): BlogPost | null {
  const db = getDb()
  return (db.prepare('SELECT * FROM posts WHERE id = ?').get(id) as BlogPost) ?? null
}

export function getAdjacentPosts(publishedAt: number): {
  prev: Pick<BlogPost, 'slug' | 'title'> | null
  next: Pick<BlogPost, 'slug' | 'title'> | null
} {
  const db = getDb()
  const prev = (db
    .prepare(
      "SELECT slug, title FROM posts WHERE status = 'published' AND published_at < ? ORDER BY published_at DESC LIMIT 1",
    )
    .get(publishedAt) as Pick<BlogPost, 'slug' | 'title'> | undefined) ?? null
  const next = (db
    .prepare(
      "SELECT slug, title FROM posts WHERE status = 'published' AND published_at > ? ORDER BY published_at ASC LIMIT 1",
    )
    .get(publishedAt) as Pick<BlogPost, 'slug' | 'title'> | undefined) ?? null
  return { prev, next }
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------
export function createPost(input: CreatePostInput): BlogPost {
  const db = getDb()
  const now = Date.now()

  const baseSlug = input.slug
    ? slugify(input.slug)
    : slugify(input.title)
  const slug = ensureUniqueSlug(baseSlug)

  const readTime = input.read_time_minutes ?? estimateReadTime(input.content)
  const status = input.status ?? 'draft'
  const publishedAt = status === 'published' ? now : null

  const result = db
    .prepare(
      `INSERT INTO posts
        (slug, title, excerpt, content, author, category, status,
         cover_image, read_time_minutes, created_at, updated_at, published_at)
       VALUES
        (@slug, @title, @excerpt, @content, @author, @category, @status,
         @cover_image, @read_time_minutes, @created_at, @updated_at, @published_at)`,
    )
    .run({
      slug,
      title: input.title,
      excerpt: input.excerpt,
      content: input.content,
      author: input.author ?? 'Core Team',
      category: input.category,
      status,
      cover_image: input.cover_image ?? null,
      read_time_minutes: readTime,
      created_at: now,
      updated_at: now,
      published_at: publishedAt,
    })

  return getPostById(result.lastInsertRowid as number)!
}

export function updatePost(input: UpdatePostInput): BlogPost {
  const db = getDb()
  const existing = getPostById(input.id)
  if (!existing) throw new Error(`Post ${input.id} not found`)

  const now = Date.now()

  // Handle slug change
  let slug = existing.slug
  if (input.slug !== undefined && input.slug !== existing.slug) {
    slug = ensureUniqueSlug(slugify(input.slug), input.id)
  } else if (input.title && !input.slug) {
    // Only auto-reslug if title changes AND no explicit slug given
    // Keep existing slug to avoid breaking links
  }

  // Handle status change → set published_at
  let publishedAt = existing.published_at
  if (input.status === 'published' && existing.status !== 'published') {
    publishedAt = now
  } else if (input.status === 'draft') {
    publishedAt = null
  }

  const content = input.content ?? existing.content
  const readTime =
    input.read_time_minutes ??
    (input.content ? estimateReadTime(content) : existing.read_time_minutes)

  db.prepare(
    `UPDATE posts SET
      slug              = @slug,
      title             = @title,
      excerpt           = @excerpt,
      content           = @content,
      author            = @author,
      category          = @category,
      status            = @status,
      cover_image       = @cover_image,
      read_time_minutes = @read_time_minutes,
      updated_at        = @updated_at,
      published_at      = @published_at
     WHERE id = @id`,
  ).run({
    id: input.id,
    slug,
    title: input.title ?? existing.title,
    excerpt: input.excerpt ?? existing.excerpt,
    content,
    author: input.author ?? existing.author,
    category: input.category ?? existing.category,
    status: input.status ?? existing.status,
    cover_image: input.cover_image !== undefined ? input.cover_image : existing.cover_image,
    read_time_minutes: readTime,
    updated_at: now,
    published_at: publishedAt,
  })

  return getPostById(input.id)!
}

export function deletePost(id: number): void {
  const db = getDb()
  db.prepare('DELETE FROM posts WHERE id = ?').run(id)
}

export function togglePublish(id: number): BlogPost {
  const post = getPostById(id)
  if (!post) throw new Error(`Post ${id} not found`)
  return updatePost({
    id,
    status: post.status === 'published' ? 'draft' : 'published',
  })
}
