'use client'

import { useEffect, useState } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { PostEditor } from '@/components/blog/post-editor'
import { Loader2 } from 'lucide-react'
import type { BlogPost } from '@/lib/blog/types'

export default function EditPostPage() {
  const { id } = useParams<{ id: string }>()
  const router = useRouter()
  const [post, setPost] = useState<BlogPost | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    fetch(`/api/blog/posts/${id}`)
      .then((res) => {
        if (res.status === 401) { router.push('/community/blog/admin'); return null }
        if (!res.ok) { setError('Post not found'); return null }
        return res.json()
      })
      .then((data) => {
        if (data) setPost(data.post)
      })
      .catch(() => setError('Failed to load post'))
      .finally(() => setLoading(false))
  }, [id, router])

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error || !post) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background text-foreground">
        <p className="text-muted-foreground">{error || 'Post not found'}</p>
      </div>
    )
  }

  return <PostEditor post={post} />
}
