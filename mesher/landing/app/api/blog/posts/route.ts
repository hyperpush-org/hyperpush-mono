import { NextRequest, NextResponse } from 'next/server'
import { isAdminRequest } from '@/lib/blog/auth'
import { listPosts, createPost } from '@/lib/blog/queries'

export const runtime = 'nodejs'
export const dynamic = 'force-dynamic'

export async function GET(req: NextRequest) {
  const isAdmin = isAdminRequest(req)
  const { searchParams } = new URL(req.url)
  const status = searchParams.get('status') as 'draft' | 'published' | 'all' | null

  // Admin-only request requires authentication
  if (status === 'all' && !isAdmin) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const posts = listPosts({
    status: isAdmin && status ? status : 'published',
    limit: Number(searchParams.get('limit') ?? 50),
    offset: Number(searchParams.get('offset') ?? 0),
  })

  return NextResponse.json({ posts })
}

export async function POST(req: NextRequest) {
  if (!isAdminRequest(req)) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  try {
    const body = await req.json()
    const post = createPost(body)
    return NextResponse.json({ post }, { status: 201 })
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : 'Failed to create post' },
      { status: 400 },
    )
  }
}
