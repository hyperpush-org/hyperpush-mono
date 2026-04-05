import { NextRequest, NextResponse } from 'next/server'
import { getAdminSecret, setAdminCookie, clearAdminCookie } from '@/lib/blog/auth'

export const runtime = 'nodejs'
export const dynamic = 'force-dynamic'

export async function POST(req: NextRequest) {
  try {
    const { password } = await req.json()
    if (!password || password !== getAdminSecret()) {
      // Constant-time-ish comparison avoidance via same message
      return NextResponse.json({ error: 'Invalid credentials' }, { status: 401 })
    }
    await setAdminCookie()
    return NextResponse.json({ ok: true })
  } catch {
    return NextResponse.json({ error: 'Bad request' }, { status: 400 })
  }
}

export async function DELETE(_req: NextRequest) {
  await clearAdminCookie()
  return NextResponse.json({ ok: true })
}
