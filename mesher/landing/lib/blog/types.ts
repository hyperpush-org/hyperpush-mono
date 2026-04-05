export interface BlogPost {
  id: number
  slug: string
  title: string
  excerpt: string
  content: string // stored as HTML from TipTap
  author: string
  category: string
  status: 'draft' | 'published'
  cover_image: string | null
  read_time_minutes: number
  created_at: number // Unix ms
  updated_at: number // Unix ms
  published_at: number | null // Unix ms
}

export interface CreatePostInput {
  slug?: string
  title: string
  excerpt: string
  content: string
  author?: string
  category: string
  status?: 'draft' | 'published'
  cover_image?: string | null
  read_time_minutes?: number
}

export interface UpdatePostInput {
  id: number
  slug?: string
  title?: string
  excerpt?: string
  content?: string
  author?: string
  category?: string
  status?: 'draft' | 'published'
  cover_image?: string | null
  read_time_minutes?: number
}

export const CATEGORIES = [
  'Announcement',
  'Engineering',
  'Deep Dive',
  'Technical',
  'Product',
  'Community',
] as const

export type Category = (typeof CATEGORIES)[number]
