'use client'

import { useEffect, useState, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { motion } from 'framer-motion'
import { CommunityLayout } from '@/components/community/community-layout'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import {
  PlusCircle, Edit2, Trash2, Eye, EyeOff,
  LogOut, Clock, Calendar, FileText, Globe,
  AlertTriangle, Loader2,
} from 'lucide-react'
import type { BlogPost } from '@/lib/blog/types'
import { cn } from '@/lib/utils'

// ── Login form ───────────────────────────────────────────────────────────────
function LoginForm({ onSuccess }: { onSuccess: () => void }) {
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')
    try {
      const res = await fetch('/api/blog/auth', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password }),
      })
      if (!res.ok) {
        setError('Invalid password')
      } else {
        onSuccess()
      }
    } catch {
      setError('Network error')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex min-h-[28rem] items-center justify-center">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="w-full max-w-sm rounded-xl border border-border bg-card p-8"
      >
        <div className="flex items-center gap-3 mb-6">
          <div className="w-8 h-8 rounded-lg bg-accent/10 flex items-center justify-center">
            <FileText className="w-4 h-4 text-accent" />
          </div>
          <div>
            <h2 className="font-semibold text-sm">Blog Admin</h2>
            <p className="text-xs text-muted-foreground">Enter your admin password</p>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Admin password"
              autoFocus
              className={cn(
                'w-full px-3 py-2 rounded-lg border text-sm bg-input',
                'placeholder:text-muted-foreground outline-none',
                'focus:ring-1 focus:ring-ring transition-shadow',
                error ? 'border-destructive' : 'border-border',
              )}
            />
            {error && (
              <p className="mt-1.5 text-xs text-destructive flex items-center gap-1">
                <AlertTriangle className="w-3 h-3" />
                {error}
              </p>
            )}
          </div>
          <Button type="submit" className="w-full" disabled={loading || !password}>
            {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : 'Sign in'}
          </Button>
        </form>
      </motion.div>
    </div>
  )
}

// ── Delete confirm dialog ────────────────────────────────────────────────────
function DeleteDialog({
  post,
  onConfirm,
  onCancel,
}: {
  post: BlogPost
  onConfirm: () => void
  onCancel: () => void
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="w-full max-w-sm rounded-xl border border-border bg-card p-6 shadow-xl mx-4"
      >
        <div className="flex items-center gap-3 mb-3">
          <div className="w-8 h-8 rounded-full bg-destructive/10 flex items-center justify-center">
            <AlertTriangle className="w-4 h-4 text-destructive" />
          </div>
          <h3 className="font-semibold">Delete post?</h3>
        </div>
        <p className="text-sm text-muted-foreground mb-5">
          &ldquo;{post.title}&rdquo; will be permanently deleted. This cannot be undone.
        </p>
        <div className="flex gap-3">
          <Button variant="outline" size="sm" className="flex-1" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="destructive" size="sm" className="flex-1" onClick={onConfirm}>
            Delete
          </Button>
        </div>
      </motion.div>
    </div>
  )
}

// ── Post row ─────────────────────────────────────────────────────────────────
function PostRow({
  post,
  onTogglePublish,
  onDelete,
}: {
  post: BlogPost
  onTogglePublish: (id: number) => void
  onDelete: (post: BlogPost) => void
}) {
  const publishedDate = post.published_at
    ? new Date(post.published_at).toLocaleDateString('en-US', {
        month: 'short', day: 'numeric', year: 'numeric',
      })
    : null

  return (
    <motion.div
      layout
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="flex items-start gap-3 px-4 py-3.5 rounded-lg border border-border hover:border-border/80 bg-card transition-colors"
    >
      {/* Status dot */}
      <div className="mt-1.5 shrink-0">
        <div
          className={cn(
            'w-2 h-2 rounded-full',
            post.status === 'published' ? 'bg-accent' : 'bg-muted-foreground/50',
          )}
        />
      </div>

      {/* Main */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-0.5 flex-wrap">
          <p className="font-medium text-sm truncate">{post.title}</p>
          <Badge
            variant="outline"
            className={cn(
              'text-xs shrink-0',
              post.status === 'published'
                ? 'border-accent/30 text-accent bg-accent/5'
                : 'border-border text-muted-foreground',
            )}
          >
            {post.status === 'published' ? (
              <span className="flex items-center gap-1"><Globe className="w-2.5 h-2.5" /> Published</span>
            ) : (
              <span className="flex items-center gap-1"><FileText className="w-2.5 h-2.5" /> Draft</span>
            )}
          </Badge>
          <Badge variant="outline" className="text-xs text-muted-foreground shrink-0">
            {post.category}
          </Badge>
        </div>
        <p className="text-xs text-muted-foreground flex items-center gap-3 mt-1">
          <span className="flex items-center gap-1">
            <Clock className="w-3 h-3" />
            {post.read_time_minutes} min
          </span>
          {publishedDate && (
            <span className="flex items-center gap-1">
              <Calendar className="w-3 h-3" />
              {publishedDate}
            </span>
          )}
          <span className="font-mono opacity-60">/{post.slug}</span>
        </p>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        {post.status === 'published' && (
          <Button variant="ghost" size="icon-sm" asChild title="View post">
            <Link href={`/community/blog/${post.slug}`} target="_blank">
              <Eye className="w-3.5 h-3.5" />
            </Link>
          </Button>
        )}
        <Button
          variant="ghost"
          size="icon-sm"
          title={post.status === 'published' ? 'Unpublish' : 'Publish'}
          onClick={() => onTogglePublish(post.id)}
        >
          {post.status === 'published'
            ? <EyeOff className="w-3.5 h-3.5" />
            : <Globe className="w-3.5 h-3.5" />
          }
        </Button>
        <Button variant="ghost" size="icon-sm" asChild title="Edit">
          <Link href={`/community/blog/admin/edit/${post.id}`}>
            <Edit2 className="w-3.5 h-3.5" />
          </Link>
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          title="Delete"
          className="text-destructive hover:text-destructive hover:bg-destructive/10"
          onClick={() => onDelete(post)}
        >
          <Trash2 className="w-3.5 h-3.5" />
        </Button>
      </div>
    </motion.div>
  )
}

// ── Main admin page ──────────────────────────────────────────────────────────
export default function AdminPage() {
  const router = useRouter()
  const [authed, setAuthed] = useState<boolean | null>(null)
  const [posts, setPosts] = useState<BlogPost[]>([])
  const [loading, setLoading] = useState(false)
  const [deleting, setDeleting] = useState<BlogPost | null>(null)

  const fetchPosts = useCallback(async () => {
    setLoading(true)
    try {
      const res = await fetch('/api/blog/posts?status=all')
      if (res.status === 401) { setAuthed(false); return }
      const data = await res.json()
      setPosts(data.posts ?? [])
      setAuthed(true)
    } catch {
      // network error
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchPosts()
  }, [fetchPosts])

  const handleTogglePublish = async (id: number) => {
    try {
      const res = await fetch(`/api/blog/posts/${id}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'toggle_publish' }),
      })
      if (!res.ok) return
      const data = await res.json()
      setPosts((p) => p.map((post) => (post.id === id ? data.post : post)))
    } catch {}
  }

  const handleDelete = async () => {
    if (!deleting) return
    try {
      await fetch(`/api/blog/posts/${deleting.id}`, { method: 'DELETE' })
      setPosts((p) => p.filter((post) => post.id !== deleting.id))
    } catch {}
    setDeleting(null)
  }

  const handleLogout = async () => {
    await fetch('/api/blog/auth', { method: 'DELETE' })
    setAuthed(false)
    setPosts([])
  }

  // Still checking auth
  if (authed === null) {
    return (
      <CommunityLayout title="Blog Admin">
        <div className="flex items-center justify-center min-h-[20rem]">
          <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
        </div>
      </CommunityLayout>
    )
  }

  // Not authenticated
  if (!authed) {
    return (
      <CommunityLayout title="Blog Admin">
        <LoginForm onSuccess={fetchPosts} />
      </CommunityLayout>
    )
  }

  const published = posts.filter((p) => p.status === 'published')
  const drafts = posts.filter((p) => p.status === 'draft')

  return (
    <>
      {deleting && (
        <DeleteDialog
          post={deleting}
          onConfirm={handleDelete}
          onCancel={() => setDeleting(null)}
        />
      )}

      <CommunityLayout title="Blog Admin">
        {/* Header row */}
        <div className="flex items-center justify-between mb-8 flex-wrap gap-3">
          <div className="flex items-center gap-3 text-sm text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <Globe className="w-3.5 h-3.5 text-accent" />
              {published.length} published
            </span>
            <span>·</span>
            <span className="flex items-center gap-1.5">
              <FileText className="w-3.5 h-3.5" />
              {drafts.length} draft{drafts.length !== 1 ? 's' : ''}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={handleLogout} className="gap-1.5">
              <LogOut className="w-3.5 h-3.5" />
              Sign out
            </Button>
            <Button size="sm" asChild className="gap-1.5">
              <Link href="/community/blog/admin/new">
                <PlusCircle className="w-3.5 h-3.5" />
                New post
              </Link>
            </Button>
          </div>
        </div>

        {loading ? (
          <div className="flex items-center justify-center min-h-[12rem]">
            <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
          </div>
        ) : posts.length === 0 ? (
          <div className="flex flex-col items-center justify-center min-h-[16rem] text-center gap-3">
            <p className="text-3xl">✍️</p>
            <p className="font-medium">No posts yet</p>
            <p className="text-sm text-muted-foreground">Create your first post to get started.</p>
            <Button asChild size="sm" className="mt-2">
              <Link href="/community/blog/admin/new">
                <PlusCircle className="w-3.5 h-3.5 mr-1.5" />
                New post
              </Link>
            </Button>
          </div>
        ) : (
          <div className="space-y-2">
            {posts.map((post) => (
              <PostRow
                key={post.id}
                post={post}
                onTogglePublish={handleTogglePublish}
                onDelete={setDeleting}
              />
            ))}
          </div>
        )}
      </CommunityLayout>
    </>
  )
}
