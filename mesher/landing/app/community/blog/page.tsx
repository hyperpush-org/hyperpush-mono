import { CommunityLayout } from '@/components/community/community-layout'
import { PostCard } from '@/components/blog/post-card'
import { listPosts } from '@/lib/blog/queries'
import { CATEGORIES } from '@/lib/blog/types'
import Link from 'next/link'
import type { Metadata } from 'next'

export const dynamic = 'force-dynamic'

export const metadata: Metadata = {
  title: 'Blog',
  description:
    'Engineering deep dives, product updates, and the thinking behind hyperpush.',
}

export default async function BlogPage({
  searchParams,
}: {
  searchParams?: Promise<{ category?: string }>
}) {
  const resolved = await searchParams
  const category = resolved?.category
  const posts = listPosts({ status: 'published', limit: 100 })
  const filtered = category
    ? posts.filter((p) => p.category === category)
    : posts

  return (
    <CommunityLayout
      title="Blog"
      subtitle="Engineering deep dives, product updates, and the thinking behind hyperpush."
    >
      {/* Category filter */}
      {posts.length > 0 && (
        <div className="flex flex-wrap gap-2 mb-8">
          <Link
            href="/community/blog"
            className={`px-3 py-1 text-xs rounded-full border transition-colors ${
              !category
                ? 'border-accent/40 bg-accent/10 text-accent'
                : 'border-border text-muted-foreground hover:text-foreground hover:border-border/80'
            }`}
          >
            All
          </Link>
          {CATEGORIES.filter((c) => posts.some((p) => p.category === c)).map((c) => (
            <Link
              key={c}
              href={`/community/blog?category=${encodeURIComponent(c)}`}
              className={`px-3 py-1 text-xs rounded-full border transition-colors ${
                category === c
                  ? 'border-accent/40 bg-accent/10 text-accent'
                  : 'border-border text-muted-foreground hover:text-foreground hover:border-border/80'
              }`}
            >
              {c}
            </Link>
          ))}
        </div>
      )}

      {/* Posts grid */}
      {filtered.length === 0 ? (
        <div className="flex min-h-[20rem] items-center justify-center text-center">
          <div>
            <p className="text-5xl mb-4">✍️</p>
            <p className="font-semibold mb-2">
              {category ? `No posts in "${category}" yet` : 'No posts yet'}
            </p>
            <p className="text-sm text-muted-foreground">
              Engineering notes and product updates are on the way.
            </p>
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
          {filtered.map((post, i) => (
            <PostCard key={post.id} post={post} index={i} />
          ))}
        </div>
      )}
    </CommunityLayout>
  )
}
