'use client'

import Link from 'next/link'
import Image from 'next/image'
import { motion } from 'framer-motion'
import { Badge } from '@/components/ui/badge'
import { Clock, Calendar, ArrowRight } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { BlogPost } from '@/lib/blog/types'

const CATEGORY_STYLES: Record<string, string> = {
  Announcement: 'bg-accent/20 text-accent border-accent/30',
  Technical:    'bg-chart-2/20 text-chart-2 border-chart-2/30',
  'Deep Dive':  'bg-chart-4/20 text-chart-4 border-chart-4/30',
  Engineering:  'bg-chart-3/20 text-chart-3 border-chart-3/30',
  Product:      'bg-chart-5/20 text-chart-5 border-chart-5/30',
  Community:    'bg-primary/10 text-primary border-primary/20',
}

function formatDate(ts: number) {
  return new Date(ts).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

interface PostCardProps {
  post: BlogPost
  index?: number
  showDraftBadge?: boolean
}

export function PostCard({ post, index = 0, showDraftBadge = false }: PostCardProps) {
  const categoryStyle = CATEGORY_STYLES[post.category] ?? 'bg-muted text-muted-foreground border-border'

  return (
    <motion.article
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, delay: index * 0.06 }}
    >
      <Link
        href={`/community/blog/${post.slug}`}
        className={cn(
          'group flex flex-col rounded-xl border border-border bg-card',
          'transition-all duration-200 hover:border-accent/30 hover:bg-card/80',
          'overflow-hidden',
        )}
      >
        {/* Cover image */}
        {post.cover_image && (
          <div className="relative w-full aspect-[16/9] overflow-hidden bg-muted">
            <Image
              src={post.cover_image}
              alt={post.title}
              fill
              className="object-cover transition-transform duration-500 group-hover:scale-105"
              unoptimized
            />
          </div>
        )}

        <div className="flex flex-col flex-1 p-5">
          {/* Badges row */}
          <div className="flex items-center gap-2 mb-3 flex-wrap">
            <Badge className={cn('text-xs', categoryStyle)}>
              {post.category}
            </Badge>
            {showDraftBadge && post.status === 'draft' && (
              <Badge variant="outline" className="text-xs text-muted-foreground border-dashed">
                Draft
              </Badge>
            )}
          </div>

          {/* Title */}
          <h2 className="font-semibold text-base leading-snug mb-2 text-foreground group-hover:text-accent transition-colors line-clamp-2">
            {post.title}
          </h2>

          {/* Excerpt */}
          <p className="text-sm text-muted-foreground leading-relaxed line-clamp-3 flex-1 mb-4">
            {post.excerpt}
          </p>

          {/* Footer */}
          <div className="flex items-center justify-between mt-auto pt-3 border-t border-border/50">
            <div className="flex items-center gap-3 text-xs text-muted-foreground">
              <span className="flex items-center gap-1">
                <Calendar className="w-3 h-3" />
                {formatDate(post.published_at ?? post.created_at)}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="w-3 h-3" />
                {post.read_time_minutes} min
              </span>
            </div>
            <ArrowRight className="w-4 h-4 text-muted-foreground group-hover:text-accent group-hover:translate-x-0.5 transition-all" />
          </div>
        </div>
      </Link>
    </motion.article>
  )
}
