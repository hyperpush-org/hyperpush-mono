'use client'

import { useState, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { RichEditor } from '@/components/blog/editor'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { CATEGORIES } from '@/lib/blog/types'
import { cn } from '@/lib/utils'
import { ArrowLeft, Save, Globe, FileText, ImageIcon, X, Loader2 } from 'lucide-react'
import type { BlogPost } from '@/lib/blog/types'

interface PostEditorProps {
  post?: BlogPost // present = edit mode, absent = new
}

export function PostEditor({ post }: PostEditorProps) {
  const router = useRouter()

  const [title, setTitle] = useState(post?.title ?? '')
  const [slug, setSlug] = useState(post?.slug ?? '')
  const [excerpt, setExcerpt] = useState(post?.excerpt ?? '')
  const [content, setContent] = useState(post?.content ?? '')
  const [author, setAuthor] = useState(post?.author ?? 'Core Team')
  const [category, setCategory] = useState<string>(post?.category ?? 'Engineering')
  const [coverImage, setCoverImage] = useState(post?.cover_image ?? '')
  const [status, setStatus] = useState<'draft' | 'published'>(post?.status ?? 'draft')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')

  // Auto-slug from title (new post only)
  const handleTitleChange = useCallback(
    (val: string) => {
      setTitle(val)
      if (!post) {
        setSlug(
          val
            .toLowerCase()
            .replace(/[^\w\s-]/g, '')
            .replace(/\s+/g, '-')
            .replace(/-+/g, '-')
            .slice(0, 80),
        )
      }
    },
    [post],
  )

  // Image upload to API
  const uploadImage = useCallback(async (file: File): Promise<string> => {
    const form = new FormData()
    form.append('file', file)
    const res = await fetch('/api/blog/upload', { method: 'POST', body: form })
    if (!res.ok) throw new Error('Upload failed')
    const data = await res.json()
    return data.url as string
  }, [])

  const save = useCallback(
    async (publishStatus?: 'draft' | 'published') => {
      setError('')
      setSaving(true)
      const finalStatus = publishStatus ?? status
      try {
        const payload = {
          title: title.trim(),
          slug: slug.trim() || undefined,
          excerpt: excerpt.trim(),
          content,
          author,
          category,
          status: finalStatus,
          cover_image: coverImage.trim() || null,
        }

        const url = post ? `/api/blog/posts/${post.id}` : '/api/blog/posts'
        const method = post ? 'PATCH' : 'POST'

        const res = await fetch(url, {
          method,
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload),
        })

        const data = await res.json()
        if (!res.ok) {
          setError(data.error ?? 'Save failed')
          return
        }

        setStatus(finalStatus)
        router.push('/community/blog/admin')
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error')
      } finally {
        setSaving(false)
      }
    },
    [title, slug, excerpt, content, author, category, coverImage, status, post, router],
  )

  const isNew = !post

  return (
    <div className="min-h-screen bg-background text-foreground">
      {/* Top bar */}
      <div className="sticky top-0 z-30 border-b border-border bg-background/95 backdrop-blur-sm">
        <div className="max-w-5xl mx-auto px-6 h-14 flex items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="icon-sm" asChild>
              <Link href="/community/blog/admin">
                <ArrowLeft className="w-4 h-4" />
              </Link>
            </Button>
            <span className="text-sm font-medium hidden sm:block">
              {isNew ? 'New post' : 'Edit post'}
            </span>
            <Badge
              variant="outline"
              className={cn(
                'text-xs',
                status === 'published'
                  ? 'border-accent/30 text-accent bg-accent/5'
                  : 'border-border text-muted-foreground',
              )}
            >
              {status === 'published' ? (
                <span className="flex items-center gap-1">
                  <Globe className="w-2.5 h-2.5" /> Published
                </span>
              ) : (
                <span className="flex items-center gap-1">
                  <FileText className="w-2.5 h-2.5" /> Draft
                </span>
              )}
            </Badge>
          </div>

          <div className="flex items-center gap-2">
            {error && (
              <p className="text-xs text-destructive hidden md:block">{error}</p>
            )}
            <Button
              variant="outline"
              size="sm"
              disabled={saving}
              onClick={() => save('draft')}
            >
              {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Save className="w-3.5 h-3.5" />}
              Save draft
            </Button>
            <Button
              size="sm"
              disabled={saving}
              onClick={() => save('published')}
            >
              {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Globe className="w-3.5 h-3.5" />}
              {status === 'published' ? 'Update' : 'Publish'}
            </Button>
          </div>
        </div>
      </div>

      {/* Body */}
      <div className="max-w-5xl mx-auto px-6 py-8 grid grid-cols-1 lg:grid-cols-[1fr_280px] gap-8">
        {/* Editor column */}
        <div className="space-y-4">
          {error && (
            <div className="flex items-center gap-2 px-4 py-3 rounded-lg bg-destructive/10 border border-destructive/30 text-sm text-destructive">
              {error}
            </div>
          )}

          {/* Title */}
          <div>
            <input
              value={title}
              onChange={(e) => handleTitleChange(e.target.value)}
              placeholder="Post title…"
              className="w-full text-3xl font-bold bg-transparent outline-none text-foreground placeholder:text-muted-foreground/50 border-b border-transparent focus:border-border pb-2 transition-colors"
            />
          </div>

          {/* Excerpt */}
          <div>
            <textarea
              value={excerpt}
              onChange={(e) => setExcerpt(e.target.value)}
              placeholder="Short excerpt / summary (shown in post cards)…"
              rows={2}
              className={cn(
                'w-full text-base bg-transparent outline-none resize-none',
                'text-muted-foreground placeholder:text-muted-foreground/40',
                'border-b border-transparent focus:border-border pb-2 transition-colors',
              )}
            />
          </div>

          {/* Rich editor */}
          <RichEditor
            content={content}
            onChange={setContent}
            placeholder="Write your post here… Markdown shortcuts work: ## heading, **bold**, `code`, > quote, - list…"
            minHeight={500}
            uploadImage={uploadImage}
          />
        </div>

        {/* Sidebar / meta */}
        <div className="space-y-5 lg:sticky lg:top-20 lg:self-start">
          {/* Status */}
          <div className="rounded-lg border border-border bg-card p-4 space-y-3">
            <h3 className="text-xs font-mono text-accent uppercase tracking-wider">Status</h3>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setStatus('draft')}
                className={cn(
                  'flex-1 py-1.5 text-xs rounded-md border transition-colors',
                  status === 'draft'
                    ? 'border-accent/30 bg-accent/5 text-accent'
                    : 'border-border text-muted-foreground hover:text-foreground',
                )}
              >
                <FileText className="w-3 h-3 inline mr-1" />
                Draft
              </button>
              <button
                type="button"
                onClick={() => setStatus('published')}
                className={cn(
                  'flex-1 py-1.5 text-xs rounded-md border transition-colors',
                  status === 'published'
                    ? 'border-accent/30 bg-accent/5 text-accent'
                    : 'border-border text-muted-foreground hover:text-foreground',
                )}
              >
                <Globe className="w-3 h-3 inline mr-1" />
                Published
              </button>
            </div>
          </div>

          {/* Category */}
          <div className="rounded-lg border border-border bg-card p-4 space-y-3">
            <h3 className="text-xs font-mono text-accent uppercase tracking-wider">Category</h3>
            <div className="grid grid-cols-2 gap-1.5">
              {CATEGORIES.map((c) => (
                <button
                  key={c}
                  type="button"
                  onClick={() => setCategory(c)}
                  className={cn(
                    'text-xs py-1 px-2 rounded-md border transition-colors text-left truncate',
                    category === c
                      ? 'border-accent/30 bg-accent/5 text-accent'
                      : 'border-border text-muted-foreground hover:text-foreground',
                  )}
                >
                  {c}
                </button>
              ))}
            </div>
          </div>

          {/* Author */}
          <div className="rounded-lg border border-border bg-card p-4 space-y-2">
            <h3 className="text-xs font-mono text-accent uppercase tracking-wider">Author</h3>
            <input
              value={author}
              onChange={(e) => setAuthor(e.target.value)}
              placeholder="Core Team"
              className="w-full text-sm bg-input border border-border rounded-md px-3 py-1.5 outline-none focus:ring-1 focus:ring-ring transition-shadow"
            />
          </div>

          {/* Slug */}
          <div className="rounded-lg border border-border bg-card p-4 space-y-2">
            <h3 className="text-xs font-mono text-accent uppercase tracking-wider">Slug</h3>
            <div className="flex items-center gap-1 text-xs text-muted-foreground mb-1">
              <span className="font-mono opacity-60">/blog/</span>
            </div>
            <input
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="auto-generated-from-title"
              className="w-full text-sm font-mono bg-input border border-border rounded-md px-3 py-1.5 outline-none focus:ring-1 focus:ring-ring transition-shadow"
            />
          </div>

          {/* Cover image */}
          <div className="rounded-lg border border-border bg-card p-4 space-y-2">
            <h3 className="text-xs font-mono text-accent uppercase tracking-wider">Cover Image</h3>
            <div className="flex gap-2">
              <input
                value={coverImage}
                onChange={(e) => setCoverImage(e.target.value)}
                placeholder="https://... or /api/blog/image/..."
                className="flex-1 text-xs bg-input border border-border rounded-md px-3 py-1.5 outline-none focus:ring-1 focus:ring-ring transition-shadow min-w-0"
              />
              {coverImage && (
                <button
                  type="button"
                  onClick={() => setCoverImage('')}
                  className="text-muted-foreground hover:text-foreground transition-colors"
                >
                  <X className="w-4 h-4" />
                </button>
              )}
            </div>
            {coverImage && (
              <div className="relative aspect-video rounded-md overflow-hidden border border-border mt-2">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img src={coverImage} alt="Cover preview" className="w-full h-full object-cover" />
              </div>
            )}
            {!coverImage && (
              <div className="flex items-center justify-center aspect-video rounded-md border border-dashed border-border bg-muted/20 text-muted-foreground">
                <div className="text-center">
                  <ImageIcon className="w-5 h-5 mx-auto mb-1 opacity-40" />
                  <p className="text-xs opacity-50">No cover image</p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
