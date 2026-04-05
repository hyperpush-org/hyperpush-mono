import { notFound } from 'next/navigation'
import Link from 'next/link'
import Image from 'next/image'
import type { Metadata } from 'next'
import { CommunityLayout } from '@/components/community/community-layout'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { WaitlistButton } from '@/components/landing/waitlist-dialog'
import { ArrowLeft, ArrowRight, Clock, Calendar } from 'lucide-react'
import { getPostBySlug, getAdjacentPosts } from '@/lib/blog/queries'
import { DISCORD_URL } from '@/lib/external-links'
import { ShareButton } from '@/components/blog/share-button'

export const dynamic = 'force-dynamic'

type Props = { params: Promise<{ slug: string }> }

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params
  const post = getPostBySlug(slug)
  if (!post) return { title: 'Post Not Found' }
  return {
    title: post.title,
    description: post.excerpt,
    openGraph: post.cover_image
      ? { images: [{ url: post.cover_image }] }
      : undefined,
  }
}

const CATEGORY_STYLES: Record<string, string> = {
  Announcement: 'bg-accent/20 text-accent border-accent/30',
  Technical:    'bg-chart-2/20 text-chart-2 border-chart-2/30',
  'Deep Dive':  'bg-chart-4/20 text-chart-4 border-chart-4/30',
  Engineering:  'bg-chart-3/20 text-chart-3 border-chart-3/30',
  Product:      'bg-chart-5/20 text-chart-5 border-chart-5/30',
  Community:    'bg-primary/10 text-primary border-primary/20',
}

export default async function BlogPostPage({ params }: Props) {
  const { slug } = await params
  const post = getPostBySlug(slug)

  if (!post || post.status !== 'published') notFound()

  const { prev, next } = getAdjacentPosts(post.published_at!)
  const categoryStyle = CATEGORY_STYLES[post.category] ?? 'bg-muted text-muted-foreground'

  const publishedDate = new Date(post.published_at!).toLocaleDateString('en-US', {
    year: 'numeric', month: 'long', day: 'numeric',
  })

  return (
    <CommunityLayout title="Blog">
      <article className="max-w-3xl">
        {/* Back */}
        <Link
          href="/community/blog"
          className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors mb-8"
        >
          <ArrowLeft className="w-4 h-4" />
          All posts
        </Link>

        {/* Cover image */}
        {post.cover_image && (
          <div className="relative w-full aspect-[21/9] rounded-xl overflow-hidden border border-border mb-8">
            <Image
              src={post.cover_image}
              alt={post.title}
              fill
              className="object-cover"
              priority
              unoptimized
            />
          </div>
        )}

        {/* Header */}
        <div className="flex items-center gap-2.5 mb-4">
          <Badge className={`text-xs ${categoryStyle}`}>
            {post.category}
          </Badge>
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <Clock className="w-3 h-3" />
            {post.read_time_minutes} min read
          </span>
        </div>

        <h1 className="text-3xl md:text-4xl font-bold tracking-tight mb-4 text-balance">
          {post.title}
        </h1>

        <p className="text-lg text-muted-foreground mb-6 text-pretty leading-relaxed">
          {post.excerpt}
        </p>

        <div className="flex items-center justify-between pb-8 border-b border-border mb-10">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-xs font-mono text-muted-foreground">
              {post.author.split(' ').map((w) => w[0]).join('').slice(0, 2).toUpperCase()}
            </div>
            <div className="text-sm">
              <p className="font-medium">{post.author}</p>
              <p className="text-muted-foreground flex items-center gap-1">
                <Calendar className="w-3 h-3" />
                {publishedDate}
              </p>
            </div>
          </div>
          <ShareButton title={post.title} />
        </div>

        {/* Content */}
        <div
          className="blog-content"
          dangerouslySetInnerHTML={{ __html: post.content }}
        />

        {/* Prev / Next */}
        {(prev || next) && (
          <nav className="mt-16 pt-8 border-t border-border grid grid-cols-2 gap-4">
            {prev ? (
              <Link
                href={`/community/blog/${prev.slug}`}
                className="group flex flex-col gap-1 p-4 rounded-xl border border-border hover:border-accent/30 transition-colors"
              >
                <span className="text-xs text-muted-foreground flex items-center gap-1">
                  <ArrowLeft className="w-3 h-3" /> Previous
                </span>
                <span className="text-sm font-medium line-clamp-2 group-hover:text-accent transition-colors">
                  {prev.title}
                </span>
              </Link>
            ) : (
              <div />
            )}
            {next ? (
              <Link
                href={`/community/blog/${next.slug}`}
                className="group flex flex-col gap-1 p-4 rounded-xl border border-border hover:border-accent/30 transition-colors text-right ml-auto w-full"
              >
                <span className="text-xs text-muted-foreground flex items-center gap-1 justify-end">
                  Next <ArrowRight className="w-3 h-3" />
                </span>
                <span className="text-sm font-medium line-clamp-2 group-hover:text-accent transition-colors">
                  {next.title}
                </span>
              </Link>
            ) : (
              <div />
            )}
          </nav>
        )}

        {/* CTA */}
        <div className="mt-16 rounded-xl border border-accent/20 bg-accent/5 p-6 text-center">
          <p className="font-semibold mb-2">Join the hyperpush community</p>
          <p className="text-sm text-muted-foreground mb-4">
            Get updates on new features, engineering deep-dives, and community events.
          </p>
          <div className="flex items-center justify-center gap-3">
            <WaitlistButton size="sm">Join Waitlist</WaitlistButton>
            <Button size="sm" variant="outline" asChild>
              <a href={DISCORD_URL} target="_blank" rel="noopener noreferrer">
                Discord
              </a>
            </Button>
          </div>
        </div>
      </article>
    </CommunityLayout>
  )
}
