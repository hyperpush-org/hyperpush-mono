import { NextRequest, NextResponse } from 'next/server'
import { isAdminRequest } from '@/lib/blog/auth'
import { getPostById, updatePost, deletePost, togglePublish } from '@/lib/blog/queries'

export const runtime = 'nodejs'
export const dynamic = 'force-dynamic'

type Ctx = { params: Promise<{ id: string }> }

export async function GET(_req: NextRequest, { params }: Ctx) {
  const { id } = await params
  const post = getPostById(Number(id))
  if (!post) return NextResponse.json({ error: 'Not found' }, { status: 404 })
  return NextResponse.json({ post })
}

export async function PATCH(req: NextRequest, { params }: Ctx) {
  if (!isAdminRequest(req)) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }
  const { id } = await params
  try {
    const body = await req.json()

    // Handle toggle-publish shorthand
    if (body.action === 'toggle_publish') {
      const post = togglePublish(Number(id))
      return NextResponse.json({ post })
    }

    const post = updatePost({ id: Number(id), ...body })
    return NextResponse.json({ post })
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : 'Failed to update post' },
      { status: 400 },
    )
  }
}

export async function DELETE(req: NextRequest, { params }: Ctx) {
  if (!isAdminRequest(req)) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }
  const { id } = await params
  try {
    deletePost(Number(id))
    return NextResponse.json({ ok: true })
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : 'Failed to delete post' },
      { status: 400 },
    )
  }
}
