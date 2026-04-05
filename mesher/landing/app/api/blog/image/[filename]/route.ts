import { NextRequest, NextResponse } from 'next/server'
import fs from 'fs'
import path from 'path'

export const runtime = 'nodejs'
export const dynamic = 'force-dynamic'

const CONTENT_TYPES: Record<string, string> = {
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  png: 'image/png',
  gif: 'image/gif',
  webp: 'image/webp',
  svg: 'image/svg+xml',
  avif: 'image/avif',
}

function uploadsDir(): string {
  return process.env.NODE_ENV === 'production'
    ? '/data/uploads'
    : path.join(process.cwd(), 'data', 'uploads')
}

type Ctx = { params: Promise<{ filename: string }> }

export async function GET(_req: NextRequest, { params }: Ctx) {
  const { filename } = await params

  // Sanitize — prevent path traversal
  const safe = path.basename(filename)
  if (safe !== filename || safe.startsWith('.')) {
    return NextResponse.json({ error: 'Invalid filename' }, { status: 400 })
  }

  const filePath = path.join(uploadsDir(), safe)

  try {
    const buf = fs.readFileSync(filePath)
    const ext = path.extname(safe).slice(1).toLowerCase()
    const ct = CONTENT_TYPES[ext] ?? 'application/octet-stream'

    return new NextResponse(buf, {
      headers: {
        'Content-Type': ct,
        'Cache-Control': 'public, max-age=31536000, immutable',
      },
    })
  } catch {
    return NextResponse.json({ error: 'Not found' }, { status: 404 })
  }
}
