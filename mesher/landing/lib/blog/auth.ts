import { cookies } from 'next/headers'
import { NextRequest } from 'next/server'

const COOKIE_NAME = 'blog_admin'
const COOKIE_MAX_AGE = 60 * 60 * 24 * 7 // 7 days

export function getAdminSecret(): string {
  return process.env.BLOG_ADMIN_SECRET ?? 'dev-secret-change-me'
}

// ── Server-side (RSC / Route Handlers) ──────────────────────────────────────

export async function isAdminFromCookies(): Promise<boolean> {
  const jar = await cookies()
  const val = jar.get(COOKIE_NAME)?.value
  return val === getAdminSecret()
}

export async function setAdminCookie(): Promise<void> {
  const jar = await cookies()
  jar.set(COOKIE_NAME, getAdminSecret(), {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'strict',
    maxAge: COOKIE_MAX_AGE,
    path: '/',
  })
}

export async function clearAdminCookie(): Promise<void> {
  const jar = await cookies()
  jar.delete(COOKIE_NAME)
}

// ── Route Handler helper (reads from Request object) ────────────────────────

export function isAdminRequest(req: NextRequest): boolean {
  const val = req.cookies.get(COOKIE_NAME)?.value
  return val === getAdminSecret()
}
